use crate::forms::InteractiveSlot;
use crate::panels::{self, Target};
use crate::store::Store;
use crate::{rebar, workflow};
use ocs_plugin_api::host::{CommandStep, EntityType, Handle, HostApi, InteractiveCommand};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub(crate) enum Measurement {
    Item(workflow::MeasureOptions),
    Rebar(rebar::CaptureOptions),
}

impl Measurement {
    fn panel(&self) -> &'static str {
        match self {
            Self::Item(_) => panels::ITEMS,
            Self::Rebar(_) => panels::REBAR,
        }
    }

    fn objects(&self) -> &'static str {
        match self {
            Self::Item(_) => "ölçülecek nesneleri",
            Self::Rebar(_) => "demir yazılarını (TEXT / MTEXT)",
        }
    }
}

struct Request {
    id: u64,
    tab: u64,
    revision: u64,
    target: Target,
    measurement: Measurement,
}

struct Gather {
    _slot: InteractiveSlot,
    request: Option<Request>,
    handles: Vec<Handle>,
}

struct Ready {
    request: Request,
    handles: Vec<Handle>,
}

// Interactive callbacks run without a document host. Enter hands the selected
// handles to a regular dispatch before the CAD host clears the selection.
static READY: Mutex<Option<Ready>> = Mutex::new(None);

impl InteractiveCommand for Gather {
    fn prompt(&self) -> String {
        let Some(request) = &self.request else {
            return String::new();
        };
        format!(
            "Çizimde {} seçin ve Enter'a basın. Esc ile iptal ({} seçili).",
            request.measurement.objects(),
            self.handles.len()
        )
    }

    fn on_point(&mut self, _: [f64; 3]) -> CommandStep {
        CommandStep::NeedPoint
    }

    fn is_selection_gathering(&self) -> bool {
        true
    }

    fn accepts_drag_selection(&self) -> bool {
        true
    }

    fn on_selection_complete(&mut self, handles: Vec<Handle>) -> CommandStep {
        self.handles = handles;
        CommandStep::NeedPoint
    }

    fn on_enter(&mut self) -> CommandStep {
        if self.handles.is_empty() {
            return CommandStep::NeedPoint;
        }
        let Some(request) = self.request.take() else {
            return CommandStep::Cancel;
        };
        let id = request.id;
        *READY.lock().unwrap() = Some(Ready {
            request,
            handles: std::mem::take(&mut self.handles),
        });
        CommandStep::Dispatch(format!("METRAJ_SECIM_TAMAMLA {id}"))
    }
}

pub(crate) fn start(
    host: &mut dyn HostApi,
    store: &mut Store,
    measurement: Measurement,
) -> Result<(), String> {
    let handles = host.selection();
    if !handles.is_empty() {
        return measure(host, store, measurement, &handles);
    }

    static NEXT: AtomicU64 = AtomicU64::new(1);
    let request = Request {
        id: NEXT.fetch_add(1, Ordering::Relaxed),
        tab: host.tab_id(),
        revision: store.stamp(),
        target: panels::target_snapshot(),
        measurement,
    };
    host.start_interactive(Box::new(Gather {
        _slot: InteractiveSlot::acquire()?,
        request: Some(request),
        handles: Vec::new(),
    }));
    Ok(())
}

pub(crate) fn finish(host: &mut dyn HostApi, id: u64) -> Result<(), String> {
    let ready = {
        let mut pending = READY.lock().unwrap();
        if !pending.as_ref().is_some_and(|ready| ready.request.id == id) {
            return Err("Metraj seçim oturumu bulunamadı.".into());
        }
        pending.take().unwrap()
    };
    let mut store = Store::load()?;
    if host.tab_id() != ready.request.tab {
        return Err("Çizim sekmesi değişti. Ölçümü yeniden başlatın.".into());
    }
    if store.stamp() != ready.request.revision || panels::target_snapshot() != ready.request.target
    {
        return Err("Etkin proje, grup veya kalem değişti. Ölçümü yeniden başlatın.".into());
    }
    measure(host, &mut store, ready.request.measurement, &ready.handles)
}

fn measure(
    host: &mut dyn HostApi,
    store: &mut Store,
    measurement: Measurement,
    handles: &[Handle],
) -> Result<(), String> {
    let panel = measurement.panel();
    let mut entities = Vec::with_capacity(handles.len());
    for batch in handles.chunks(256) {
        let rows = host.entities_batch(batch)?;
        if rows.len() != batch.len() {
            return Err("Çizimdeki seçim kümesi okunamadı.".into());
        }
        for (&handle, row) in batch.iter().zip(rows) {
            let entity: EntityType = row.ok_or_else(|| {
                format!("{:X} numaralı nesne çizimde bulunamadı.", handle.value())
            })?;
            entities.push(entity);
        }
    }
    match measurement {
        Measurement::Item(options) => workflow::measure(host, store, options, entities)?,
        Measurement::Rebar(options) => rebar::capture(host, store, options, entities)?,
    }
    panels::refresh(host, store)?;
    panels::reveal(host, panel)
}
