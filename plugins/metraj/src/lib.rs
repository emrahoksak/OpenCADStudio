mod forms;
mod import;
mod manage;
mod rebar;
mod store;
mod workflow;

use ocs_plugin_api::dialog::{DialogControl, DialogDef, DialogResponse, DialogValue, PanelDef};
use ocs_plugin_api::host::{BuiltinPlugin, HostApi};
use ocs_plugin_api::manifest::{ApiVersion, PluginManifest};
use ocs_plugin_api::ribbon::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef};
use std::sync::OnceLock;
use store::{Group, Project, Store};

const PANEL_ID: &str = "metraj_panel";
const ACTIVE_GROUP: &str = "active_group";
const SELECT_GROUP: &str = "Proje ve grup seçin";

static MANIFEST: PluginManifest = PluginManifest {
    id: "metraj.opencad",
    name: "Metraj",
    version: "0.1.0",
    description: "Proje bazlı metraj yönetimi",
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
                tools: vec![RibbonItem::LargeTool(ToolDef {
                    id: "METRAJ",
                    label: "Metraj Paneli",
                    icon: IconKind::Glyph("▤"),
                    event: ModuleEvent::Command("METRAJ".to_owned()),
                })],
            }]
        })
    }
}

fn button(id: &str, label: &str, action: &str) -> DialogControl {
    DialogControl::Button {
        id: id.into(),
        label: label.into(),
        action: Some(action.into()),
        align_h: None,
        width: None,
    }
}
fn panel() -> PanelDef {
    PanelDef::new(PANEL_ID, "Metraj")
        .with_width(320.0)
        .with_control(workflow::choice(
            ACTIVE_GROUP,
            "Aktif grup",
            vec![SELECT_GROUP.into()],
        ))
        .with_control(button("create_project", "Yeni Proje", "METRAJ_PROJE"))
        .with_control(button("import", "SQLite Projelerini Aktar", "METRAJ_AKTAR"))
        .with_control(button("create_group", "Grup Oluştur", "METRAJ_GRUP"))
        .with_control(button("list", "Projeler ve Gruplar", "METRAJ_LISTE"))
        .with_control(button("item", "Metraj Kalemi Oluştur", "METRAJ_KALEM"))
        .with_control(button("measure", "Seçimden Metraj Al", "METRAJ_OLC"))
        .with_control(button("manual", "Manuel Metraj", "METRAJ_MANUEL"))
        .with_control(button("rebar", "Seçimden Demir Al", "METRAJ_DEMIR"))
        .with_control(button(
            "manual_rebar",
            "Manuel Demir",
            "METRAJ_DEMIR_MANUEL",
        ))
        .with_control(button(
            "settings",
            "Demir Eşleştirme Ayarları",
            "METRAJ_AYAR",
        ))
        .with_control(button("summary", "Grup İcmali", "METRAJ_ICMAL"))
        .with_control(button("records", "Kayıtları Gör / Sil", "METRAJ_KAYIT"))
        .with_control(button("manage", "Düzenle / Sil", "METRAJ_DUZENLE"))
        .with_control(button(
            "item_edit",
            "Kalemi Düzenle / Çoğalt",
            "METRAJ_KALEM_DUZENLE",
        ))
        .with_control(button(
            "export",
            "Grup İcmalini Excel'e Aktar",
            "METRAJ_EXCEL",
        ))
}

fn group_choices(store: &Store) -> Vec<((usize, usize), String)> {
    store
        .projects
        .iter()
        .enumerate()
        .flat_map(|(pi, project)| {
            project
                .groups
                .iter()
                .enumerate()
                .map(move |(gi, group)| ((pi, gi), format!("{} / {}", project.name, group.name)))
        })
        .collect()
}

fn refresh_panel(host: &mut dyn HostApi) {
    let Ok(store) = Store::load() else {
        return;
    };
    let old = host.panel_snapshot(PANEL_ID).ok().and_then(|snapshot| {
        match snapshot.values.get(ACTIVE_GROUP)? {
            DialogValue::Choice { text, .. } => Some(text.clone()),
            _ => None,
        }
    });
    let mut options = vec![SELECT_GROUP.into()];
    options.extend(group_choices(&store).into_iter().map(|(_, label)| label));
    let selected_index = old
        .and_then(|label| options.iter().position(|option| *option == label))
        .unwrap_or(if options.len() == 2 { 1 } else { 0 });
    host.update_panel(
        PANEL_ID,
        [(
            ACTIVE_GROUP.into(),
            DialogValue::ChoiceOptions {
                options,
                selected_index,
            },
        )]
        .into(),
    );
}

fn input(
    host: &mut dyn HostApi,
    title: &str,
    project_names: Option<Vec<String>>,
) -> Option<(String, usize)> {
    let mut dialog = DialogDef::new(title)
        .submit_label("Kaydet")
        .cancel_label("Vazgeç")
        .with_control(DialogControl::Text {
            id: "name".into(),
            label: "Ad".into(),
            default: String::new(),
        });
    if let Some(names) = project_names {
        dialog = dialog.with_control(DialogControl::Choice {
            id: "project".into(),
            label: "Proje".into(),
            options: names,
            default_index: 0,
        });
    }
    let DialogResponse::Submitted(values) = host.show_dialog(dialog) else {
        return None;
    };
    let name = match values.get("name") {
        Some(DialogValue::Text(value)) => value.trim().to_owned(),
        _ => String::new(),
    };
    let project = match values.get("project") {
        Some(DialogValue::Choice { index, .. }) => *index,
        _ => 0,
    };
    Some((name, project))
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
        host.register_panel(panel());
        refresh_panel(host);
    }
    fn dispatch(&self, host: &mut dyn HostApi, command: &str) -> bool {
        let command = command.trim().to_ascii_uppercase();
        if command == "METRAJ" {
            if !host.toggle_panel(PANEL_ID, Some(true))
                && !(host.register_panel(panel()) && host.toggle_panel(PANEL_ID, Some(true)))
            {
                host.push_error("Metraj paneli açılamadı.");
            } else {
                refresh_panel(host);
            }
            return true;
        }
        if !matches!(
            command.as_str(),
            "METRAJ_PROJE"
                | "METRAJ_AKTAR"
                | "METRAJ_GRUP"
                | "METRAJ_LISTE"
                | "METRAJ_KALEM"
                | "METRAJ_OLC"
                | "METRAJ_MANUEL"
                | "METRAJ_DEMIR"
                | "METRAJ_AYAR"
                | "METRAJ_DEMIR_MANUEL"
                | "METRAJ_KAYIT"
                | "METRAJ_DUZENLE"
                | "METRAJ_KALEM_DUZENLE"
                | "METRAJ_EXCEL"
                | "METRAJ_ICMAL"
                | "METRAJ_DEVAM"
        ) {
            return false;
        }
        forms::run(host, &command, execute);
        true
    }
}
fn execute(host: &mut dyn HostApi, command: &str) -> Result<(), String> {
    match command {
        "METRAJ_PROJE" => create_project(host),
        "METRAJ_AKTAR" => import::run(host),
        "METRAJ_GRUP" => create_group(host),
        "METRAJ_LISTE" => list(host),
        "METRAJ_KALEM" => workflow::create_item(host),
        "METRAJ_OLC" => workflow::measure(host),
        "METRAJ_MANUEL" => workflow::manual(host),
        "METRAJ_DEMIR" => rebar::capture(host),
        "METRAJ_AYAR" => rebar::settings(host),
        "METRAJ_DEMIR_MANUEL" => manage::manual_rebar(host),
        "METRAJ_KAYIT" => manage::records(host),
        "METRAJ_DUZENLE" => manage::edit(host),
        "METRAJ_KALEM_DUZENLE" => manage::edit_item(host),
        "METRAJ_EXCEL" => manage::export(host),
        "METRAJ_ICMAL" => workflow::summary(host),
        _ => Err("Bilinmeyen metraj komutu.".into()),
    }
}
fn create_project(host: &mut dyn HostApi) -> Result<(), String> {
    let mut store = Store::load()?;
    let Some((name, _)) = input(host, "Yeni Proje", None) else {
        return Ok(());
    };
    if name.is_empty() {
        return Err("Proje adı boş olamaz.".into());
    }
    if store
        .projects
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case(&name))
    {
        return Err("Bu proje adı zaten var.".into());
    }
    store.projects.push(Project {
        name,
        groups: Vec::new(),
        settings: Default::default(),
    });
    store.save()?;
    host.push_info("Proje oluşturuldu.");
    Ok(())
}
fn create_group(host: &mut dyn HostApi) -> Result<(), String> {
    let mut store = Store::load()?;
    if store.projects.is_empty() {
        return Err("Önce bir proje oluşturun.".into());
    }
    let names = store.projects.iter().map(|p| p.name.clone()).collect();
    let Some((name, index)) = input(host, "Yeni Grup", Some(names)) else {
        return Ok(());
    };
    if name.is_empty() {
        return Err("Grup adı boş olamaz.".into());
    }
    let project = store.projects.get_mut(index).ok_or("Proje bulunamadı.")?;
    if project
        .groups
        .iter()
        .any(|g| g.name.eq_ignore_ascii_case(&name))
    {
        return Err("Bu grup adı zaten var.".into());
    }
    project.groups.push(Group {
        name,
        items: Vec::new(),
        rebar: Vec::new(),
    });
    store.save()?;
    host.push_info("Grup oluşturuldu.");
    Ok(())
}
fn list(host: &mut dyn HostApi) -> Result<(), String> {
    let store = Store::load()?;
    let mut text = String::new();
    for project in &store.projects {
        text.push_str(&format!("\n{}", project.name));
        for group in &project.groups {
            text.push_str(&format!("\n  • {}", group.name));
        }
    }
    if text.is_empty() {
        text = "Henüz proje yok.".into();
    }
    host.show_dialog(
        DialogDef::new("Projeler ve Gruplar")
            .with_control(DialogControl::InfoText {
                text,
                align_h: None,
            })
            .submit_label("Kapat"),
    );
    Ok(())
}
ocs_plugin_api::export_plugin!(MetrajPlugin);
