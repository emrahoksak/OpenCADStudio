mod forms;
mod import;
mod manage;
mod panels;
mod rebar;
mod selection;
mod store;
mod workflow;

use ocs_plugin_api::host::{BuiltinPlugin, HostApi};
use ocs_plugin_api::manifest::{ApiVersion, PluginManifest};
use ocs_plugin_api::ribbon::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef};
use std::sync::OnceLock;
use store::Store;

static MANIFEST: PluginManifest = PluginManifest {
    id: "metraj.opencad",
    name: "Metraj",
    version: "0.2.2",
    description: "Proje, kalem ve demir metrajı çalışma panelleri",
    api_version: ApiVersion::CURRENT,
    ribbon_order: 55,
    xdata_apps: &[],
    command_prefixes: &["METRAJ"],
};

struct MetrajModule;
impl CadModule for MetrajModule {
    fn id(&self) -> &'static str {
        "metraj"
    }
    fn title(&self) -> &'static str {
        "Metraj"
    }
    fn ribbon_groups(&self) -> &[RibbonGroup] {
        static GROUPS: OnceLock<Vec<RibbonGroup>> = OnceLock::new();
        GROUPS.get_or_init(|| {
            vec![RibbonGroup {
                title: "Metraj",
                tools: [
                    ("METRAJ_AYARLAR", "Metraj Ayarları", "⚙"),
                    ("METRAJ", "Metraj", "▤"),
                    ("METRAJ_DEMIR_PANEL", "Demir Metrajı", "Ø"),
                ]
                .into_iter()
                .map(|(id, label, icon)| {
                    RibbonItem::LargeTool(ToolDef {
                        id,
                        label,
                        icon: IconKind::Glyph(icon),
                        event: ModuleEvent::Command(id.to_owned()),
                    })
                })
                .collect(),
            }]
        })
    }
}

struct MetrajPlugin;
impl BuiltinPlugin for MetrajPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn ribbon(&self) -> Box<dyn CadModule> {
        Box::new(MetrajModule)
    }
    fn on_load(&mut self, host: &mut dyn HostApi) {
        if let Err(error) = panels::initialize(host) {
            host.push_error(&error);
        }
    }
    fn dispatch(&self, host: &mut dyn HostApi, command: &str) -> bool {
        let command = command.trim().to_ascii_uppercase();
        if let Some(result) = panels::dispatch(host, &command) {
            if let Err(error) = result {
                host.push_error(&error);
            }
            return true;
        }
        let verb = command.split_whitespace().next().unwrap_or_default();
        if !matches!(
            verb,
            "METRAJ_AKTAR"
                | "METRAJ_DUZENLE"
                | "METRAJ_KALEM_DUZENLE"
                | "METRAJ_KALEM_SIL"
                | "METRAJ_KAYIT_SIL"
                | "METRAJ_DEMIR_SIL"
                | "METRAJ_EXCEL"
                | "METRAJ_MANUEL"
                | "METRAJ_DEMIR_MANUEL"
                | "METRAJ_ICMAL"
                | "METRAJ_DEVAM"
        ) {
            return false;
        }
        forms::run(host, &command, execute);
        true
    }
}

fn execute(host: &mut dyn HostApi, command: &str, store: &mut Store) -> Result<(), String> {
    match command {
        "METRAJ_AKTAR" => import::run(host, store),
        "METRAJ_DUZENLE" => manage::edit(host, store),
        "METRAJ_KALEM_DUZENLE" => manage::edit_item(host, store),
        "METRAJ_KALEM_SIL" => manage::delete_item(host, store),
        "METRAJ_KAYIT_SIL" => manage::delete_record(host, store, false),
        "METRAJ_DEMIR_SIL" => manage::delete_record(host, store, true),
        "METRAJ_EXCEL" => manage::export(host, store),
        "METRAJ_MANUEL" => workflow::manual(host, store),
        "METRAJ_DEMIR_MANUEL" => manage::manual_rebar(host, store),
        "METRAJ_ICMAL" => workflow::summary(host, store),
        _ => Err("Bilinmeyen metraj komutu.".into()),
    }
}

ocs_plugin_api::export_plugin!(MetrajPlugin);
