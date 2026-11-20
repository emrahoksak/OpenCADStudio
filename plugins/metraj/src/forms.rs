use crate::store::Store;
use ocs_plugin_api::dialog::{DialogDef, DialogResponse, PanelSnapshot};
use ocs_plugin_api::host::{
    CommandStep, Document, DocumentReader, EntityType, ExtendedDataRecord, Handle, HostApi,
    InteractiveCommand, LayerConfig,
};
use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

// The runner reuses interactive command IDs across dispatches. Keep one
// Metraj command alive at a time, including when the user switches drawings.
static INTERACTIVE_ACTIVE: AtomicBool = AtomicBool::new(false);

pub(crate) struct InteractiveSlot;

impl InteractiveSlot {
    pub(crate) fn acquire() -> Result<Self, String> {
        INTERACTIVE_ACTIVE
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| Self)
            .map_err(|_| {
                "Diğer çizimdeki Metraj işlemini tamamlayın veya Esc ile iptal edin.".into()
            })
    }
}

impl Drop for InteractiveSlot {
    fn drop(&mut self) {
        INTERACTIVE_ACTIVE.store(false, Ordering::Release);
    }
}

struct Session {
    command: String,
    responses: Vec<DialogResponse>,
    tab: u64,
    revision: Option<Vec<u8>>,
    panel: Option<PanelSnapshot>,
    target: crate::panels::Target,
}
fn session() -> &'static Mutex<Option<Session>> {
    static SESSION: OnceLock<Mutex<Option<Session>>> = OnceLock::new();
    SESSION.get_or_init(|| Mutex::new(None))
}

pub fn run(
    host: &mut dyn HostApi,
    command: &str,
    execute: fn(&mut dyn HostApi, &str, &mut Store) -> Result<(), String>,
) {
    let mut store = match Store::load() {
        Ok(store) => store,
        Err(error) => {
            host.push_error(&error);
            return;
        }
    };
    let state = if command == "METRAJ_DEVAM" {
        match session().lock().unwrap().take() {
            Some(s) => s,
            None => {
                host.push_error("Metraj işlem oturumu bulunamadı.");
                return;
            }
        }
    } else {
        if let Err(error) = crate::panels::require_current(host, &store) {
            host.push_error(&error);
            return;
        }
        let panel = match command {
            "METRAJ_KAYIT_SIL" => host.panel_snapshot(crate::panels::ITEMS).ok(),
            "METRAJ_DEMIR_SIL" => host.panel_snapshot(crate::panels::REBAR).ok(),
            "METRAJ_PROJE_SIL" | "METRAJ_GRUP" | "METRAJ_GRUP_SIL" => {
                host.panel_snapshot(crate::panels::SETTINGS).ok()
            }
            _ => None,
        };
        Session {
            command: command.into(),
            responses: Vec::new(),
            tab: host.tab_id(),
            revision: store.original_revision().map(<[u8]>::to_vec),
            panel,
            target: crate::panels::target_snapshot(),
        }
    };
    if state.tab != host.tab_id() {
        host.push_error("Çizim sekmesi değişti; metraj işlemi iptal edildi.");
        return;
    }
    if command == "METRAJ_DEVAM" && store.original_revision() != state.revision.as_deref() {
        host.push_error("Metraj verileri değişti. İşlemi yeniden başlatın.");
        return;
    }
    if state.target != crate::panels::target_snapshot() {
        host.push_error("Etkin proje, grup veya kalem değişti; metraj işlemi iptal edildi.");
        return;
    }
    let (outcome, pending) = {
        let mut proxy = ReplayHost {
            host,
            answers: &state.responses,
            cursor: 0,
            pending: None,
            panel: state.panel.as_ref(),
        };
        let outcome = execute(&mut proxy, &state.command, &mut store);
        (outcome, proxy.pending)
    };
    if let Some(dialog) = pending {
        let slot = match InteractiveSlot::acquire() {
            Ok(slot) => slot,
            Err(error) => {
                host.push_error(&error);
                return;
            }
        };
        *session().lock().unwrap() = Some(state);
        host.start_interactive(Box::new(Form {
            dialog,
            closed: false,
            _slot: slot,
        }));
    } else if let Err(error) = outcome {
        host.push_error(&error);
    } else if matches!(
        state.command.as_str(),
        "METRAJ_AKTAR"
            | "METRAJ_DUZENLE"
            | "METRAJ_PROJE"
            | "METRAJ_PROJE_SIL"
            | "METRAJ_GRUP"
            | "METRAJ_GRUP_SIL"
            | "METRAJ_KALEM_DUZENLE"
            | "METRAJ_KALEM_SIL"
            | "METRAJ_KAYIT_SIL"
            | "METRAJ_DEMIR_SIL"
            | "METRAJ_MANUEL"
            | "METRAJ_DEMIR_MANUEL"
    ) {
        let refresh = match state.command.as_str() {
            "METRAJ_AKTAR" | "METRAJ_DUZENLE" | "METRAJ_PROJE" | "METRAJ_PROJE_SIL"
            | "METRAJ_GRUP" | "METRAJ_GRUP_SIL" => crate::panels::refresh(host, &store),
            "METRAJ_DEMIR_SIL" | "METRAJ_DEMIR_MANUEL" => {
                crate::panels::refresh_measurements(host, &store, crate::panels::REBAR)
            }
            _ => crate::panels::refresh_measurements(host, &store, crate::panels::ITEMS),
        };
        if let Err(error) = refresh {
            host.push_error(&error);
            return;
        }
        if matches!(
            state.command.as_str(),
            "METRAJ_KALEM_DUZENLE" | "METRAJ_KALEM_SIL"
        ) {
            if let Err(error) = crate::panels::reveal(host, crate::panels::ITEMS) {
                host.push_error(&error);
            }
        }
    }
}
struct Form {
    dialog: DialogDef,
    closed: bool,
    _slot: InteractiveSlot,
}
impl Drop for Form {
    fn drop(&mut self) {
        if !self.closed {
            *session().lock().unwrap() = None;
        }
    }
}

impl InteractiveCommand for Form {
    fn prompt(&self) -> String {
        "Metraj formunu doldurun".into()
    }
    fn on_point(&mut self, _: [f64; 3]) -> CommandStep {
        CommandStep::NeedPoint
    }
    fn modal_dialog(&self) -> Option<DialogDef> {
        if self.closed {
            None
        } else {
            Some(self.dialog.clone())
        }
    }
    fn on_dialog_response(&mut self, response: DialogResponse) -> CommandStep {
        self.closed = true;
        let mut guard = session().lock().unwrap();
        if matches!(response, DialogResponse::Cancelled) {
            *guard = None;
            return CommandStep::Cancel;
        }
        if let Some(s) = guard.as_mut() {
            s.responses.push(response);
            CommandStep::Dispatch("METRAJ_DEVAM".into())
        } else {
            CommandStep::Cancel
        }
    }
}
struct ReplayHost<'a> {
    host: &'a mut dyn HostApi,
    answers: &'a [DialogResponse],
    cursor: usize,
    pending: Option<DialogDef>,
    panel: Option<&'a PanelSnapshot>,
}
impl HostApi for ReplayHost<'_> {
    fn tab_index(&self) -> usize {
        self.host.tab_index()
    }
    fn document(&self) -> &Document {
        self.host.document()
    }
    fn document_mut(&mut self) -> &mut Document {
        self.host.document_mut()
    }
    fn add_entity(&mut self, e: EntityType) -> Handle {
        self.host.add_entity(e)
    }
    fn add_entities(&mut self, e: Vec<EntityType>) -> Vec<Handle> {
        self.host.add_entities(e)
    }
    fn add_layer(&mut self, c: LayerConfig) -> Option<Handle> {
        self.host.add_layer(c)
    }
    fn update_entities_transaction(
        &mut self,
        label: &str,
        e: Vec<EntityType>,
    ) -> Result<(), String> {
        self.host.update_entities_transaction(label, e)
    }
    fn bump_geometry(&mut self) {
        self.host.bump_geometry()
    }
    fn read_record(&self, h: Handle, a: &str) -> Option<&ExtendedDataRecord> {
        self.host.read_record(h, a)
    }
    fn write_record(&mut self, h: Handle, r: ExtendedDataRecord) -> bool {
        self.host.write_record(h, r)
    }
    fn remove_record(&mut self, h: Handle, a: &str) -> bool {
        self.host.remove_record(h, a)
    }
    fn push_undo(&mut self, label: &str) {
        self.host.push_undo(label)
    }
    fn begin_undo_group(&mut self, label: &str) {
        self.host.begin_undo_group(label)
    }
    fn end_undo_group(&mut self) {
        self.host.end_undo_group()
    }
    fn set_dirty(&mut self) {
        self.host.set_dirty()
    }
    fn push_info(&mut self, msg: &str) {
        self.host.push_info(msg)
    }
    fn push_output(&mut self, msg: &str) {
        self.host.push_output(msg)
    }
    fn push_error(&mut self, msg: &str) {
        self.host.push_error(msg)
    }
    fn start_interactive(&mut self, c: Box<dyn InteractiveCommand>) {
        self.host.start_interactive(c)
    }
    fn plugin_state_any(&self, id: &str) -> Option<&(dyn Any + Send + Sync)> {
        self.host.plugin_state_any(id)
    }
    fn plugin_state_any_mut(&mut self, id: &str) -> Option<&mut (dyn Any + Send + Sync)> {
        self.host.plugin_state_any_mut(id)
    }
    fn ensure_plugin_state_any(
        &mut self,
        id: &'static str,
        init: &mut dyn FnMut() -> Box<dyn Any + Send + Sync>,
    ) -> Option<&mut (dyn Any + Send + Sync)> {
        self.host.ensure_plugin_state_any(id, init)
    }
    fn document_reader(&self) -> Box<dyn DocumentReader + '_> {
        self.host.document_reader()
    }
    fn panel_snapshot(&self, panel_id: &str) -> Result<PanelSnapshot, String> {
        self.panel
            .filter(|panel| panel.panel_id == panel_id)
            .cloned()
            .ok_or_else(|| "Metraj paneli bulunamadı.".into())
    }
    fn show_dialog(&mut self, dialog: DialogDef) -> DialogResponse {
        if let Some(response) = self.answers.get(self.cursor) {
            self.cursor += 1;
            response.clone()
        } else {
            if self.pending.is_none() {
                self.pending = Some(dialog);
            }
            DialogResponse::Cancelled
        }
    }
}
