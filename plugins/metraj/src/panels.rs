use crate::selection::{self, Measurement};
use crate::store::{Group, Item, Project, Store};
use crate::workflow::{choice, index, string, text};
use crate::{rebar, workflow};
use ocs_plugin_api::dialog::{
    DialogControl as C, DialogValue as V, HorizontalAlign, PanelDef, PanelStyle, TextStyle,
};
use ocs_plugin_api::host::HostApi;
use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

const SETTINGS: &str = "metraj_ayar_panel";
pub(crate) const ITEMS: &str = "metraj_panel";
pub(crate) const REBAR: &str = "metraj_demir_panel";
const PAGE_SIZE: usize = 10;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Target {
    project: Option<String>,
    group: Option<String>,
    item: Option<usize>,
}
impl Target {
    fn project(&self, store: &Store) -> Option<usize> {
        let name = self.project.as_ref()?;
        store.projects.iter().position(|p| &p.name == name)
    }

    fn group(&self, store: &Store) -> Option<(usize, usize)> {
        let p = self.project(store)?;
        let name = self.group.as_ref()?;
        let g = store.projects[p]
            .groups
            .iter()
            .position(|g| &g.name == name)?;
        Some((p, g))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum View {
    #[default]
    List,
    New,
    Detail,
}

#[derive(Default)]
struct State {
    target: Target,
    revision: u64,
    view: View,
    item_page: usize,
    record_page: usize,
    rebar_page: usize,
    last_click: Option<(usize, u64, Instant)>,
    item_layout: Option<PanelDef>,
    settings_project: Option<String>,
    project_options: Vec<String>,
    group_options: Vec<String>,
}
impl State {
    fn show_list(&mut self) {
        self.target.item = None;
        self.view = View::List;
        self.record_page = 0;
        self.last_click = None;
    }

    fn select(&mut self, project: Option<String>, group: Option<String>) {
        if self.target.project == project && self.target.group == group {
            return;
        }
        self.target.project = project;
        self.target.group = group;
        self.item_page = 0;
        self.rebar_page = 0;
        self.show_list();
    }
}

fn state() -> MutexGuard<'static, State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE
        .get_or_init(|| Mutex::new(State::default()))
        .lock()
        .unwrap()
}

pub(crate) fn target_snapshot() -> Target {
    state().target.clone()
}

pub(crate) fn active_project(store: &Store) -> Result<usize, String> {
    state()
        .target
        .project(store)
        .ok_or_else(|| "Metraj Ayarları panelinde bir proje oluşturun veya seçin.".into())
}

pub(crate) fn active_group(store: &Store) -> Result<(usize, usize), String> {
    state()
        .target
        .group(store)
        .ok_or_else(|| "Metraj Ayarları panelinde çalışacağınız grubu etkinleştirin.".into())
}

pub(crate) fn active_item(group: &Group) -> Result<usize, String> {
    state()
        .target
        .item
        .filter(|&i| i < group.items.len())
        .ok_or_else(|| "Metraj panelinde bir kalem açın.".into())
}

pub(crate) fn open_item(i: usize) {
    let mut s = state();
    s.target.item = Some(i);
    s.view = View::Detail;
    s.record_page = 0;
    s.last_click = None;
}

pub(crate) fn close_item() {
    state().show_list();
}

pub(crate) fn rename_project(old: &str, new: String) {
    let mut s = state();
    if s.target.project.as_deref() == Some(old) {
        s.target.project = Some(new);
    }
}

pub(crate) fn rename_group(old: &str, new: String) {
    let mut s = state();
    if s.target.group.as_deref() == Some(old) {
        s.target.group = Some(new);
    }
}

fn panel(id: &str, title: &str, controls: Vec<C>) -> PanelDef {
    PanelDef {
        controls,
        ..PanelDef::new(id, title).with_style(PanelStyle::Palette)
    }
}

fn button(id: &str, label: &str, action: impl Into<String>) -> C {
    C::Button {
        id: id.into(),
        label: label.into(),
        action: Some(action.into()),
        align_h: None,
        width: None,
    }
}

fn item_button(i: usize, label: String, action: String) -> C {
    C::Button {
        id: format!("item_{i}"),
        label,
        action: Some(action),
        align_h: Some(HorizontalAlign::Left),
        width: None,
    }
}

fn label(id: &str) -> C {
    C::Label {
        id: id.into(),
        text: String::new(),
        align_h: None,
    }
}

fn styled(text: impl Into<String>, style: TextStyle) -> C {
    C::StyledText {
        text: text.into(),
        style,
        align_h: None,
    }
}

fn heading(text: &str) -> C {
    styled(text, TextStyle::Strong)
}

fn hint(text: &str) -> C {
    styled(text, TextStyle::Muted)
}

fn choices(id: &str, label: &str, options: &[&str]) -> C {
    choice(id, label, options.iter().map(|s| (*s).into()).collect())
}

fn table(id: &str, headers: &[&str]) -> C {
    C::Table {
        id: id.into(),
        label: None,
        headers: headers.iter().map(|s| (*s).into()).collect(),
        rows: Vec::new(),
        allow_add: false,
        allow_delete: false,
    }
}

fn dropdown(options: Vec<String>, selected_index: usize) -> V {
    V::ChoiceOptions {
        options,
        selected_index,
    }
}

fn page_buttons(key: &str) -> C {
    C::row(vec![
        button(
            &format!("{key}_prev"),
            "‹ Önceki",
            format!("METRAJ_SAYFA {key} -1"),
        ),
        label(&format!("{key}_page")),
        button(
            &format!("{key}_next"),
            "Sonraki ›",
            format!("METRAJ_SAYFA {key} 1"),
        ),
    ])
}

fn page_values(updates: &mut HashMap<String, V>, key: &str, page: &mut usize, count: usize) {
    let pages = count.div_ceil(PAGE_SIZE).max(1);
    *page = (*page).min(pages - 1);
    updates.insert(
        format!("{key}_page"),
        V::Text(format!("{} / {pages}", *page + 1)),
    );
    updates.insert(format!("{key}_prev"), V::Enabled(*page > 0));
    updates.insert(format!("{key}_next"), V::Enabled(*page + 1 < pages));
    for control in ["prev", "page", "next"] {
        updates.insert(format!("{key}_{control}:visible"), V::Visible(pages > 1));
    }
}

fn settings_panel() -> PanelDef {
    let mut controls = vec![
        label("context"),
        choices("project", "Proje", &["Proje seçin"]),
        text("new_project", "Yeni proje adı", ""),
        C::row(vec![
            button("select_project", "Projeyi Seç", "METRAJ_PROJE_SEC"),
            button("create_project", "Proje Oluştur", "METRAJ_PROJE"),
        ]),
        choices("group", "Grup", &["Grup seçin"]),
        text("new_group", "Yeni grup adı", ""),
        C::row(vec![
            button("select_group", "Grubu Etkinleştir", "METRAJ_GRUP_SEC"),
            button("create_group", "Grup Oluştur", "METRAJ_GRUP"),
        ]),
        C::row(vec![
            button("manage", "Proje / Grup Yönetimi", "METRAJ_DUZENLE"),
            button("import", "SQLite'tan Aktar", "METRAJ_AKTAR"),
        ]),
        heading("Demir eşleştirme"),
    ];
    controls.extend(rebar::settings_controls(&Default::default()));
    controls.extend([
        button("save_settings", "Demir Ayarlarını Kaydet", "METRAJ_AYAR"),
        C::row(vec![
            button("open_items", "Metraja Git", "METRAJ"),
            button("open_rebar", "Demir Metrajına Git", "METRAJ_DEMIR_PANEL"),
        ]),
    ]);
    panel(SETTINGS, "Metraj Ayarları", controls).with_width(320.0)
}

fn items_panel(controls: Vec<C>) -> PanelDef {
    panel(ITEMS, "Metraj", controls)
}

fn rebar_panel() -> PanelDef {
    panel(
        REBAR,
        "Demir Metrajı",
        vec![
            label("context"),
            label("mode_status"),
            text("multiplier", "Adet çarpanı", "1"),
            choices("sign", "İşlem", &["Metraj", "Minha"]),
            C::row(vec![
                button("capture", "Çizimden Demir Al", "METRAJ_DEMIR"),
                button("manual", "Manuel Giriş…", "METRAJ_DEMIR_MANUEL"),
            ]),
            heading("Grup icmali"),
            label("total"),
            table("by_diameter", &["Çap", "Boy (m)", "Ağırlık (kg)"]),
            heading("Demir kayıtları"),
            table(
                "records",
                &["No.", "Çap", "Adet", "Boy (cm)", "kg", "Kaynak"],
            ),
            page_buttons("DEMIR"),
            choices("delete_record", "Kayıt no.", &["Seçin"]),
            C::row(vec![
                button("settings", "Ayarlar", "METRAJ_AYARLAR"),
                button("delete", "Kaydı Sil…", "METRAJ_DEMIR_SIL"),
            ]),
        ],
    )
    .with_width(360.0)
}

pub(crate) fn initialize(host: &mut dyn HostApi) -> Result<(), String> {
    register(host, settings_panel())?;
    register(host, rebar_panel())?;
    register(host, items_panel(Vec::new()).with_width(330.0))?;
    refresh(host, &Store::load()?)
}

fn register(host: &mut dyn HostApi, panel: PanelDef) -> Result<(), String> {
    let title = panel.title.clone();
    if host.register_panel(panel) {
        Ok(())
    } else {
        Err(format!("{title} paneli kaydedilemedi."))
    }
}

pub(crate) fn reveal(host: &mut dyn HostApi, panel: &str) -> Result<(), String> {
    if host.toggle_panel(panel, Some(true)) {
        Ok(())
    } else {
        Err("Metraj paneli açılamadı.".into())
    }
}

fn context(project: Option<&Project>, group: Option<&Group>) -> String {
    match (project, group) {
        (Some(p), Some(g)) => format!("{} / {}", p.name, g.name),
        (Some(p), None) => format!("{} / Grup etkinleştirilmedi", p.name),
        _ => "Metraj Ayarları panelinde proje ve grup seçin.".into(),
    }
}

fn active_choice(options: &[String], active: Option<&str>) -> usize {
    active
        .and_then(|name| options.iter().skip(1).position(|v| v == name))
        .map_or(0, |i| i + 1)
}

fn refresh_settings(
    host: &mut dyn HostApi,
    s: &mut State,
    store: &Store,
    project: Option<&Project>,
    group: Option<&Group>,
) {
    let mut updates = HashMap::from([("context".into(), V::Text(context(project, group)))]);
    let mut projects = vec!["Proje seçin".into()];
    projects.extend(store.projects.iter().map(|p| p.name.clone()));
    let mut groups = vec!["Grup seçin".into()];
    if let Some(p) = project {
        groups.extend(p.groups.iter().map(|g| g.name.clone()));
    }
    let current_project = project.map(|p| p.name.clone());
    let project_changed = s.settings_project != current_project;
    if project_changed || s.project_options != projects {
        let selected = active_choice(&projects, s.target.project.as_deref());
        updates.insert("project".into(), dropdown(projects.clone(), selected));
        s.project_options = projects;
    }
    if project_changed || s.group_options != groups {
        let selected = active_choice(&groups, s.target.group.as_deref());
        updates.insert("group".into(), dropdown(groups.clone(), selected));
        s.group_options = groups;
    }
    let defaults = Default::default();
    if project_changed {
        updates.extend(rebar::settings_values(
            project.map_or(&defaults, |p| &p.settings),
        ));
        updates.insert("new_group".into(), V::Text(String::new()));
    }
    s.settings_project = current_project;
    for id in [
        "manage",
        "group",
        "select_group",
        "new_group",
        "create_group",
        "mode",
        "symbols",
        "tolerance",
        "mixed",
        "color",
        "color_preset",
        "box",
        "save_settings",
    ] {
        updates.insert(format!("{id}:enabled"), V::Enabled(project.is_some()));
    }
    updates.insert(
        "select_project".into(),
        V::Enabled(!store.projects.is_empty()),
    );
    host.update_panel(SETTINGS, updates);
}

fn list_controls(
    s: &mut State,
    group: &Group,
    controls: &mut Vec<C>,
    updates: &mut HashMap<String, V>,
) {
    controls.push(C::row(vec![
        button("new_item", "Yeni Metraj Kalemi", "METRAJ_KALEM_YENI"),
        button("export", "Excel'e Aktar…", "METRAJ_EXCEL"),
    ]));
    if group.items.is_empty() {
        controls.push(hint("Bu grupta henüz metraj kalemi yok."));
        return;
    }
    page_values(updates, "KALEM", &mut s.item_page, group.items.len());
    let start = s.item_page * PAGE_SIZE;
    for (i, item) in group.items.iter().enumerate().skip(start).take(PAGE_SIZE) {
        let token = item_token(&s.target, item);
        let name = item.name.replace(['\r', '\n'], " ");
        let short: String = name.chars().take(36).collect();
        let suffix = if name.chars().count() > 36 { "…" } else { "" };
        let poz = if item.poz.trim().is_empty() {
            String::new()
        } else {
            format!("{} · ", item.poz)
        };
        let total: f64 = item.records.iter().map(|r| r.quantity).sum();
        let row_label = format!(
            "{poz}{short}{suffix}   |   {total:.3} {}",
            item.unit_label()
        );
        controls.push(C::row(vec![
            item_button(i, row_label, format!("METRAJ_KALEM_TIKLA {token} {i}")),
            C::Button {
                width: Some(46.0),
                id: format!("open_{i}"),
                label: "Aç".into(),
                action: Some(format!("METRAJ_KALEM_AC {token} {i}")),
                align_h: None,
            },
        ]));
    }
    controls.push(page_buttons("KALEM"));
    controls.push(button("summary", "Grup İcmali…", "METRAJ_ICMAL"));
}

fn detail_controls(
    s: &mut State,
    item: &Item,
    controls: &mut Vec<C>,
    updates: &mut HashMap<String, V>,
) {
    controls.extend([
        button("back", "‹ Kalemler", "METRAJ_KALEM_LISTE"),
        styled(item.name.clone(), TextStyle::Strong),
        label("item_metadata"),
    ]);
    let unit = item.unit_label();
    let metadata = if item.poz.is_empty() {
        unit.to_owned()
    } else {
        format!("{} · {unit}", item.poz)
    };
    updates.insert("item_metadata".into(), V::Text(metadata));

    let modes = workflow::measurement_modes(&item.unit);
    let mut actions = Vec::new();
    if !modes.is_empty() {
        if modes.len() > 1 {
            controls.push(choices("method", "Yöntem", modes));
        }
        controls.push(choices("scale", "Çizim birimi", &["cm", "mm", "m"]));
        if item.unit != "m" {
            controls.push(text("height", "Yükseklik (m)", "3"));
        }
        controls.push(choices("sign", "İşlem", &["Metraj", "Minha"]));
        actions.push(button("measure", "Çizimden Metraj Al", "METRAJ_OLC"));
    }
    actions.push(button("manual", "Manuel Giriş…", "METRAJ_MANUEL"));

    controls.extend([
        C::row(actions),
        table("records", &["No.", "Yöntem", "Miktar", "Açıklama"]),
        page_buttons("KAYIT"),
        choices("delete_record", "Kayıt no.", &["Seçin"]),
        C::row(vec![
            button("delete", "Kaydı Sil…", "METRAJ_KAYIT_SIL"),
            button("edit_item", "Kalemi Düzenle…", "METRAJ_KALEM_DUZENLE"),
            button("delete_item", "Kalemi Sil…", "METRAJ_KALEM_SIL"),
        ]),
    ]);
    page_values(updates, "KAYIT", &mut s.record_page, item.records.len());
    let start = s.record_page * PAGE_SIZE;
    let rows = item
        .records
        .iter()
        .enumerate()
        .skip(start)
        .take(PAGE_SIZE)
        .map(|(i, r)| {
            vec![
                (i + 1).to_string(),
                r.method.clone(),
                format!("{:.4}", r.quantity),
                record_preview(&r.source),
            ]
        });
    record_values(updates, rows.collect());
}

fn record_preview(source: &str) -> String {
    let mut chars = source.chars();
    let mut preview: String = chars.by_ref().take(120).collect();
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}

fn record_values(updates: &mut HashMap<String, V>, rows: Vec<Vec<String>>) {
    let mut options = vec!["Seçin".into()];
    options.extend(rows.iter().map(|r| r[0].clone()));
    updates.insert("delete_record".into(), dropdown(options, 0));
    for id in ["delete", "delete_record"] {
        updates.insert(format!("{id}:visible"), V::Visible(!rows.is_empty()));
    }
    updates.insert("records:enabled".into(), V::Enabled(false));
    updates.insert("records".into(), V::Table(rows));
}

fn refresh_items(
    host: &mut dyn HostApi,
    s: &mut State,
    project: Option<&Project>,
    group: Option<&Group>,
) -> Result<(), String> {
    let mut controls = vec![label("context")];
    let mut updates = HashMap::from([("context".into(), V::Text(context(project, group)))]);
    if let Some(group) = group {
        if s.view == View::Detail && s.target.item.and_then(|i| group.items.get(i)).is_none() {
            s.show_list();
        }
        match s.view {
            View::List => list_controls(s, group, &mut controls, &mut updates),
            View::New => controls.extend([
                button("back", "‹ Metraj Kalemleri", "METRAJ_KALEM_LISTE"),
                heading("Yeni metraj kalemi"),
                text("name", "Kalem adı", ""),
                text("poz", "Poz no.", ""),
                choices("unit", "Birim", workflow::UNITS),
                button("create", "Kalemi Oluştur", "METRAJ_KALEM"),
            ]),
            View::Detail => {
                if let Some(item) = s.target.item.and_then(|i| group.items.get(i)) {
                    detail_controls(s, item, &mut controls, &mut updates);
                }
            }
        }
    } else {
        controls.extend([
            hint("Ölçüme başlamak için Metraj Ayarları panelinde bir grup etkinleştirin."),
            button("settings", "Metraj Ayarlarını Aç", "METRAJ_AYARLAR"),
        ]);
    }
    let new_layout = items_panel(controls);
    if s.item_layout.as_ref() != Some(&new_layout) {
        register(host, new_layout.clone())?;
        s.item_layout = Some(new_layout);
    }
    host.update_panel(ITEMS, updates);

    Ok(())
}

fn refresh_rebar(
    host: &mut dyn HostApi,
    s: &mut State,
    project: Option<&Project>,
    group: Option<&Group>,
) {
    let mut updates = HashMap::from([
        ("context".into(), V::Text(context(project, group))),
        (
            "mode_status".into(),
            V::Text(
                project
                    .map(|p| format!("Eşleştirme: {}", rebar::mode_label(p.settings.mode)))
                    .unwrap_or_default(),
            ),
        ),
    ]);
    let records = group.map_or(&[][..], |g| &g.rebar);
    let mut by_diameter = BTreeMap::<u32, (f64, f64)>::new();
    for r in records {
        let totals = by_diameter.entry(r.diameter).or_default();
        totals.0 += f64::from(r.quantity) * r.length_cm / 100.0
            * if r.weight_kg < 0.0 { -1.0 } else { 1.0 };
        totals.1 += r.weight_kg;
    }
    let kg: f64 = by_diameter.values().map(|(_, weight)| weight).sum();
    updates.insert(
        "total".into(),
        V::Text(format!(
            "Net: {kg:.3} kg  ·  {:.3} ton  ·  {} kayıt",
            kg / 1000.0,
            records.len(),
        )),
    );
    updates.insert(
        "by_diameter".into(),
        V::Table(
            by_diameter
                .iter()
                .map(|(d, (m, kg))| vec![format!("Ø{d}"), format!("{m:.2}"), format!("{kg:.3}")])
                .collect(),
        ),
    );
    page_values(&mut updates, "DEMIR", &mut s.rebar_page, records.len());
    let start = s.rebar_page * PAGE_SIZE;
    record_values(
        &mut updates,
        records
            .iter()
            .enumerate()
            .skip(start)
            .take(PAGE_SIZE)
            .map(|(i, r)| {
                vec![
                    (i + 1).to_string(),
                    format!("Ø{}", r.diameter),
                    r.quantity.to_string(),
                    format!("{:.2}", r.length_cm),
                    format!("{:.3}", r.weight_kg),
                    record_preview(&r.source),
                ]
            })
            .collect(),
    );
    for id in ["capture", "manual", "multiplier", "sign"] {
        updates.insert(format!("{id}:enabled"), V::Enabled(group.is_some()));
    }
    updates.insert("by_diameter:enabled".into(), V::Enabled(false));
    host.update_panel(REBAR, updates);
}

pub(crate) fn refresh_measurements(
    host: &mut dyn HostApi,
    store: &Store,
    panel: &str,
) -> Result<(), String> {
    let mut s = state();
    let (p, g) = s.target.group(store).ok_or("Etkin grup bulunamadı.")?;
    let project = Some(&store.projects[p]);
    let group = Some(&store.projects[p].groups[g]);
    if panel == REBAR {
        refresh_rebar(host, &mut s, project, group);
    } else {
        refresh_items(host, &mut s, project, group)?;
    }
    s.revision = store.stamp();
    Ok(())
}

pub(crate) fn refresh(host: &mut dyn HostApi, store: &Store) -> Result<(), String> {
    let mut s = state();
    s.revision = store.stamp();
    if s.target.project(store).is_none() {
        s.select(None, None);
    } else if s.target.group(store).is_none() && s.target.group.is_some() {
        let project = s.target.project.clone();
        s.select(project, None);
    }
    let project = s.target.project(store).map(|p| &store.projects[p]);
    let group = s
        .target
        .group(store)
        .map(|(p, g)| &store.projects[p].groups[g]);
    refresh_settings(host, &mut s, store, project, group);
    refresh_items(host, &mut s, project, group)?;
    refresh_rebar(host, &mut s, project, group);
    Ok(())
}

fn read_panel(host: &dyn HostApi, panel_id: &str) -> Result<HashMap<String, V>, String> {
    let snapshot = host.panel_snapshot(panel_id)?;
    if !snapshot.errors.is_empty() {
        return Err(format!(
            "Metraj paneli okunamadı: {}",
            snapshot.errors.join(" • ")
        ));
    }
    Ok(snapshot.values)
}

pub(crate) fn require_current(host: &mut dyn HostApi, store: &Store) -> Result<(), String> {
    if state().revision == store.stamp() {
        return Ok(());
    }
    reset_for_external_change(host, store)?;
    Err("Metraj verileri başka bir oturumda değişti. İşlemi yeniden başlatın.".into())
}

fn reset_for_external_change(host: &mut dyn HostApi, store: &Store) -> Result<(), String> {
    close_item();
    state().settings_project = None;
    refresh(host, store)
}

fn select_project(store: &Store, values: &HashMap<String, V>) -> Result<(), String> {
    let p = selection(values, "project", &store.projects, |p| &p.name)?;
    let mut s = state();
    if s.target.project.as_deref() != Some(p.name.as_str()) {
        s.select(Some(p.name.clone()), None);
    }
    Ok(())
}

fn project_for_group(store: &Store, values: &HashMap<String, V>) -> Result<usize, String> {
    let p = active_project(store)?;
    if let Some(V::Choice { index, text }) = values.get("project") {
        if *index == p + 1 && *text == store.projects[p].name {
            return Ok(p);
        }
    }
    Err("Grup işlemi için önce Projeyi Seç düğmesine basın.".into())
}

fn select_group(store: &Store, values: &HashMap<String, V>) -> Result<(), String> {
    let p = project_for_group(store, values)?;
    let project = &store.projects[p];
    let g = selection(values, "group", &project.groups, |g| &g.name)?;
    state().select(Some(project.name.clone()), Some(g.name.clone()));
    Ok(())
}

fn selection<'a, T>(
    values: &HashMap<String, V>,
    id: &str,
    list: &'a [T],
    name: impl Fn(&T) -> &str,
) -> Result<&'a T, String> {
    let Some(V::Choice { index, text }) = values.get(id) else {
        return Err("Listeden bir seçim yapın.".into());
    };
    index
        .checked_sub(1)
        .and_then(|i| list.get(i))
        .filter(|item| name(item) == text)
        .ok_or_else(|| "Listeden bir seçim yapın.".into())
}

fn create_project(
    host: &mut dyn HostApi,
    store: &mut Store,
    values: &HashMap<String, V>,
) -> Result<(), String> {
    let name = string(values, "new_project")?;
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
        name: name.clone(),
        groups: Vec::new(),
        settings: Default::default(),
    });
    store.save()?;
    host.push_info(&format!("Proje oluşturuldu: {name}"));
    state().select(Some(name), None);
    host.update_panel(
        SETTINGS,
        [("new_project".into(), V::Text(String::new()))].into(),
    );
    Ok(())
}

fn create_group(
    host: &mut dyn HostApi,
    store: &mut Store,
    values: &HashMap<String, V>,
) -> Result<(), String> {
    let p = project_for_group(store, values)?;
    let name = string(values, "new_group")?;
    if name.is_empty() {
        return Err("Grup adı boş olamaz.".into());
    }
    let project = &mut store.projects[p];
    if project
        .groups
        .iter()
        .any(|g| g.name.eq_ignore_ascii_case(&name))
    {
        return Err("Bu grup adı zaten var.".into());
    }
    project.groups.push(Group {
        name: name.clone(),
        items: Vec::new(),
        rebar: Vec::new(),
    });
    store.save()?;
    host.push_info(&format!("Grup oluşturuldu ve etkinleştirildi: {name}"));
    state().select(Some(store.projects[p].name.clone()), Some(name));
    host.update_panel(
        SETTINGS,
        [("new_group".into(), V::Text(String::new()))].into(),
    );
    Ok(())
}

fn item_token(target: &Target, item: &Item) -> u64 {
    let mut hasher = DefaultHasher::new();
    target.project.hash(&mut hasher);
    target.group.hash(&mut hasher);
    item.name.hash(&mut hasher);
    item.poz.hash(&mut hasher);
    item.unit.hash(&mut hasher);
    hasher.finish()
}

fn item_click(
    host: &dyn HostApi,
    store: &Store,
    args: &mut std::str::SplitWhitespace<'_>,
    direct: bool,
) -> Result<bool, String> {
    let (p, g) = active_group(store)?;
    let token = args.next().and_then(|v| v.parse::<u64>().ok());
    let row = args
        .next()
        .and_then(|v| v.parse::<usize>().ok())
        .ok_or("Kalem bulunamadı.")?;
    let item = store.projects[p].groups[g]
        .items
        .get(row)
        .ok_or("Kalem bulunamadı.")?;
    let mut s = state();
    if token != Some(item_token(&s.target, item))
        || s.view != View::List
        || row / PAGE_SIZE != s.item_page
    {
        return Err("Kalem listesi değişti. Kalemi yeniden açın.".into());
    }
    let now = Instant::now();
    let tab = host.tab_id();
    let double = s.last_click.is_some_and(|(last_row, last_tab, then)| {
        last_row == row && last_tab == tab && now.duration_since(then) <= Duration::from_millis(500)
    });
    if direct || double {
        drop(s);
        open_item(row);
        Ok(true)
    } else {
        s.last_click = Some((row, tab, now));
        Ok(false)
    }
}

fn turn_page(args: &mut std::str::SplitWhitespace<'_>) -> Result<(), String> {
    let page_id = args.next().ok_or("Sayfa bulunamadı.")?;
    let step: isize = match args.next() {
        Some("-1") => -1,
        Some("1") => 1,
        _ => return Err("Sayfa bulunamadı.".into()),
    };
    let mut s = state();
    s.last_click = None;
    let page = match page_id {
        "KALEM" => &mut s.item_page,
        "KAYIT" => &mut s.record_page,
        "DEMIR" => &mut s.rebar_page,
        _ => return Err("Sayfa bulunamadı.".into()),
    };
    *page = page.saturating_add_signed(step);
    Ok(())
}

pub(crate) fn record_index(host: &dyn HostApi, rebar: bool) -> Result<usize, String> {
    let values = read_panel(host, if rebar { REBAR } else { ITEMS })?;
    if index(&values, "delete_record")? == 0 {
        return Err("Silmek için bir kayıt numarası seçin.".into());
    }
    match values.get("delete_record") {
        Some(V::Choice { text, .. }) => text.parse::<usize>().ok().and_then(|i| i.checked_sub(1)),
        _ => None,
    }
    .ok_or_else(|| "Kayıt bulunamadı.".into())
}

pub(crate) fn dispatch(host: &mut dyn HostApi, command: &str) -> Option<Result<(), String>> {
    let mut args = command.split_whitespace();
    let verb = args.next()?;
    let panel = match verb {
        "METRAJ" | "METRAJ_KALEM" | "METRAJ_KALEM_YENI" | "METRAJ_KALEM_LISTE"
        | "METRAJ_KALEM_TIKLA" | "METRAJ_KALEM_AC" | "METRAJ_OLC" => ITEMS,
        "METRAJ_AYARLAR" | "METRAJ_PROJE" | "METRAJ_PROJE_SEC" | "METRAJ_GRUP"
        | "METRAJ_GRUP_SEC" | "METRAJ_AYAR" => SETTINGS,
        "METRAJ_DEMIR_PANEL" | "METRAJ_DEMIR" => REBAR,
        "METRAJ_SAYFA" if args.clone().next() == Some("DEMIR") => REBAR,
        "METRAJ_SAYFA" => ITEMS,
        _ => return None,
    };
    if matches!(command, "METRAJ_OLC" | "METRAJ_DEMIR") {
        if let Some(result) = selection::finish_pending(host, command) {
            return Some(result);
        }
    }
    Some((|| {
        let mut store = Store::load()?;
        let opening = matches!(verb, "METRAJ" | "METRAJ_AYARLAR" | "METRAJ_DEMIR_PANEL");
        if opening {
            if state().revision != store.stamp() {
                reset_for_external_change(host, &store)?;
            }
            return reveal(host, panel);
        }
        require_current(host, &store)?;
        if !matches!(verb, "METRAJ_KALEM_TIKLA" | "METRAJ_KALEM_AC") {
            state().last_click = None;
        }
        match verb {
            "METRAJ_PROJE_SEC" => select_project(&store, &read_panel(host, SETTINGS)?)?,
            "METRAJ_GRUP_SEC" => select_group(&store, &read_panel(host, SETTINGS)?)?,
            "METRAJ_PROJE" => {
                let values = read_panel(host, SETTINGS)?;
                create_project(host, &mut store, &values)?;
            }
            "METRAJ_GRUP" => {
                let values = read_panel(host, SETTINGS)?;
                create_group(host, &mut store, &values)?;
            }
            "METRAJ_AYAR" => {
                let p = active_project(&store)?;
                rebar::save_settings(&mut store, p, &read_panel(host, SETTINGS)?)?;
                host.update_panel(
                    SETTINGS,
                    rebar::settings_values(&store.projects[p].settings),
                );
                host.push_info("Demir eşleştirme ayarları kaydedildi.");
            }
            "METRAJ_KALEM_YENI" => {
                active_group(&store)?;
                let mut s = state();
                s.show_list();
                s.view = View::New;
            }
            "METRAJ_KALEM_LISTE" => close_item(),
            "METRAJ_SAYFA" => turn_page(&mut args)?,
            "METRAJ_KALEM_TIKLA" | "METRAJ_KALEM_AC" => {
                if !item_click(host, &store, &mut args, verb == "METRAJ_KALEM_AC")? {
                    return Ok(());
                }
            }
            "METRAJ_KALEM" => {
                let (p, g) = active_group(&store)?;
                let i = workflow::create_item(&mut store, p, g, &read_panel(host, ITEMS)?)?;
                state().item_page = i / PAGE_SIZE;
                open_item(i);
            }
            "METRAJ_OLC" => {
                let values = read_panel(host, ITEMS)?;
                let options = workflow::measure_options(&store, &values)?;
                return selection::start(host, &mut store, Measurement::Item(options));
            }
            "METRAJ_DEMIR" => {
                let values = read_panel(host, REBAR)?;
                active_group(&store)?;
                let options = rebar::capture_options(&values)?;
                return selection::start(host, &mut store, Measurement::Rebar(options));
            }
            _ => return Err("Bilinmeyen metraj komutu.".into()),
        }
        if panel == SETTINGS {
            refresh(host, &store)?;
        } else {
            refresh_measurements(host, &store, panel)?;
        }
        reveal(host, panel)
    })())
}
