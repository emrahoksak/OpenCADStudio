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
    let gather = Gather {
        _slot: InteractiveSlot::acquire()?,
        request: Some(request),
        handles: Vec::new(),
    };
    host.push_info(&gather.prompt());
    host.start_interactive(Box::new(gather));
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
        read_entities(host, batch, &mut entities)?;
    }
    match measurement {
        Measurement::Item(options) => workflow::measure(host, store, options, entities)?,
        Measurement::Rebar(options) => rebar::capture(host, store, options, entities)?,
    }
    panels::refresh_measurements(host, store, panel)?;
    panels::reveal(host, panel)
}

fn read_entities(
    host: &dyn HostApi,
    handles: &[Handle],
    entities: &mut Vec<EntityType>,
) -> Result<(), String> {
    match host.entities_batch(handles) {
        Ok(rows) if rows.len() == handles.len() => {
            for (&handle, row) in handles.iter().zip(rows) {
                let entity = row.ok_or_else(|| {
                    format!("{:X} numaralı nesne çizimde bulunamadı.", handle.value())
                })?;
                entities.push(entity);
            }
            Ok(())
        }
        Ok(_) => Err("Çizimdeki seçim kümesi okunamadı.".into()),
        Err(error) if error.starts_with("message too large:") => {
            if handles.len() == 1 {
                return Err(format!(
                    "{:X} numaralı nesnenin geometrisi aktarım sınırını aşıyor.",
                    handles[0].value()
                ));
            }
            let (left, right) = handles.split_at(handles.len() / 2);
            read_entities(host, left, entities)?;
            read_entities(host, right, entities)
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn mark_entities(
    host: &mut dyn HostApi,
    label: &str,
    entities: Vec<EntityType>,
) -> Result<(), String> {
    // Leave room for request framing. One entity can exceed this target
    // budget and is then sent on its own.
    const BATCH_BYTES: u64 = 32 * 1024 * 1024;
    let mut batch = Vec::new();
    let mut bytes = 0u64;
    for entity in entities {
        let size = bincode::serialized_size(&entity).map_err(|error| error.to_string())?;
        if !batch.is_empty() && bytes.saturating_add(size) > BATCH_BYTES {
            host.update_entities_transaction(label, std::mem::take(&mut batch))?;
            bytes = 0;
        }
        bytes += size;
        batch.push(entity);
    }
    if !batch.is_empty() {
        host.update_entities_transaction(label, batch)?;
    }
    Ok(())
}
