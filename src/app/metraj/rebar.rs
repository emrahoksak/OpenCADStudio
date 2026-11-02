use super::{
    drawing_key,
    panels::{action, choice, heading},
    send, Chrome, Message, MetrajMsg, MetrajUi, OpenCADStudio, PanelId, Project,
};
use crate::metraj::rebar::{
    read_rebar, summarize_rebar, RebarEntry, RebarMarking, RebarMode, RebarSettings, RebarText,
};
use crate::ui::style::{common::muted_style, form::field_style};
use iced::widget::{checkbox, column, pick_list, row, scrollable, text, text_input};
use iced::{Element, Fill, Task};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Section {
    Settings,
    Read,
    Manual,
    Rows,
    Summary,
}
#[derive(Debug, Clone)]
pub(crate) enum RebarMsg {
    Marking(RebarMarking),
    Color(u8),
    RecolorText(bool),
    DrawBoxes(bool),
    SaveSettings,
    Mode(RebarMode),
    Multiplier(String),
    Tolerance(String),
    Symbols(String),
    Deduction(bool),
    Quantity(String),
    Diameter(String),
    Length(String),
    Filter(String),
    Toggle(Section),
    Read,
    AddManual,
    Select(i64),
    Update,
    Remove,
    ClearEdit,
    Export,
}
pub(crate) struct RebarUi {
    pub show: bool,
    mode: RebarMode,
    multiplier: String,
    tolerance: String,
    symbols: String,
    marking: RebarMarking,
    color: u8,
    recolor_text: bool,
    draw_boxes: bool,
    deduction: bool,
    quantity: String,
    diameter: String,
    length: String,
    filter: String,
    selected: Option<i64>,
    rows: Vec<RebarEntry>,
    closed: std::collections::HashSet<Section>,
}
impl Default for RebarUi {
    fn default() -> Self {
        Self {
            show: false,
            mode: RebarMode::Mod1,
            multiplier: "1".into(),
            tolerance: "0.01".into(),
            symbols: RebarSettings::default().symbols,
            marking: RebarMarking::Color,
            color: 3,
            recolor_text: true,
            draw_boxes: false,
            deduction: false,
            quantity: "1".into(),
            diameter: "16".into(),
            length: "250".into(),
            filter: String::new(),
            selected: None,
            rows: Vec::new(),
            closed: [Section::Settings, Section::Manual].into_iter().collect(),
        }
    }
}
impl RebarUi {
    pub(super) fn load(&mut self, project: &Project) -> Result<(), String> {
        let settings = project.rebar_settings().map_err(|e| e.to_string())?;
        self.mode = settings.mode;
        self.symbols = settings.symbols;
        self.marking = settings.marking;
        self.color = settings.color;
        self.recolor_text = settings.recolor_text;
        self.draw_boxes = settings.draw_boxes;
        self.multiplier = settings.multiplier.to_string();
        self.tolerance = settings.tolerance.to_string();
        self.reload(project)
    }
    fn reload(&mut self, project: &Project) -> Result<(), String> {
        self.rows = project.rebar_rows().map_err(|e| e.to_string())?;
        Ok(())
    }
    fn settings(&self) -> Result<RebarSettings, String> {
        let settings = RebarSettings {
            marking: self.marking,
            color: self.color,
            recolor_text: self.recolor_text,
            draw_boxes: self.draw_boxes,
            mode: self.mode,
            symbols: self.symbols.clone(),
            multiplier: self
                .multiplier
                .trim()
                .parse()
                .map_err(|_| "x adet pozitif tam sayı olmalı.")?,
            tolerance: self
                .tolerance
                .trim()
                .replace(',', ".")
                .parse()
                .map_err(|_| "Tolerans sayı olmalı.")?,
        };
        settings.validate()?;
        Ok(settings)
    }
    fn manual(&self) -> Result<RebarEntry, String> {
        RebarEntry::manual(
            self.quantity
                .trim()
                .parse()
                .map_err(|_| "Adet tam sayı olmalı.")?,
            self.diameter
                .trim()
                .parse()
                .map_err(|_| "Çap tam sayı olmalı.")?,
            self.length
                .trim()
                .replace(',', ".")
                .parse()
                .map_err(|_| "Boy sayı olmalı.")?,
            self.deduction,
        )
    }
}
fn msg(message: RebarMsg) -> MetrajMsg {
    MetrajMsg::Rebar(message)
}
fn field<'a>(
    hint: &'a str,
    value: &'a str,
    message: fn(String) -> RebarMsg,
) -> Element<'a, Message> {
    text_input(hint, value)
        .on_input(move |s| send(msg(message(s))))
        .size(11)
        .padding([4, 6])
        .style(field_style)
        .into()
}
impl MetrajUi {
    pub(crate) fn rebar_view(&self, width: f32, chrome: Chrome) -> Element<'_, Message> {
        let ui = &self.rebar;
        let mut body = column![].spacing(5);
        if let Some(project) = &self.project {
            body = body
                .push(text(project.name().unwrap_or_default()).size(11))
                .push(
                    text("Mahal gerektirmez · L= boyları cm, çaplar mm.")
                        .size(10)
                        .style(muted_style),
                );
            body = body.push(heading(
                "Ayarlar".into(),
                !ui.closed.contains(&Section::Settings),
                msg(RebarMsg::Toggle(Section::Settings)),
            ));
            if !ui.closed.contains(&Section::Settings) {
                body = body
                    .push(
                        checkbox(ui.recolor_text)
                            .label("Eşleşen metinlerin rengini değiştir")
                            .size(14)
                            .text_size(11)
                            .on_toggle(|v| send(msg(RebarMsg::RecolorText(v)))),
                    )
                    .push(text("Renk modu").size(11))
                    .push(
                        pick_list(Some(ui.marking), RebarMarking::ALL, |m: &RebarMarking| {
                            m.label().to_owned()
                        })
                        .on_select(|m| send(msg(RebarMsg::Marking(m))))
                        .text_size(11)
                        .padding([4, 6])
                        .width(Fill),
                    );
                if ui.marking == RebarMarking::Color {
                    body = body.push(
                        row![
                            text("Renk").size(11).width(72),
                            pick_list(Some(ui.color), (1u8..=255).collect::<Vec<_>>(), |c: &u8| {
                                match c {
                                    1 => "Kırmızı (1)".into(),
                                    2 => "Sarı (2)".into(),
                                    3 => "Yeşil (3)".into(),
                                    4 => "Camgöbeği (4)".into(),
                                    5 => "Mavi (5)".into(),
                                    6 => "Mor (6)".into(),
                                    7 => "Beyaz (7)".into(),
                                    _ => format!("ACI {c}"),
                                }
                            })
                            .on_select(|c| send(msg(RebarMsg::Color(c))))
                            .text_size(11)
                            .padding([4, 6])
                            .width(Fill)
                        ]
                        .spacing(4)
                        .align_y(iced::Center),
                    );
                } else {
                    body = body.push(text("Her eşleşme ayrı renk alır. Aynı gruptaki adet/çap, boy ve kutu aynı renktedir.").size(10).style(muted_style));
                }
                body = body.push(
                    checkbox(ui.draw_boxes)
                        .label("Her eşleşme için bounding box çiz")
                        .size(14)
                        .text_size(11)
                        .on_toggle(|v| send(msg(RebarMsg::DrawBoxes(v)))),
                );
                if ui.draw_boxes {
                    body = body.push(text("Her eşleşme grubuna, gerçek yazı sınırlarından en küçük alanlı kutu. Ek boşluk: 0.").size(10).style(muted_style));
                }
                body = body.push(
                    row![
                        text("Çap simgesi").size(11).width(72),
                        field("ΦØøϕ", &ui.symbols, RebarMsg::Symbols)
                    ]
                    .spacing(4)
                    .align_y(iced::Center),
                );
                if matches!(ui.mode, RebarMode::Horizontal | RebarMode::Vertical) {
                    body = body
                        .push(
                            row![
                                text("Tolerans").size(11).width(72),
                                field("Çizim birimi", &ui.tolerance, RebarMsg::Tolerance)
                            ]
                            .spacing(4)
                            .align_y(iced::Center),
                        )
                        .push(
                            text("Hizalama toleransı çizim birimidir.")
                                .size(10)
                                .style(muted_style),
                        );
                }
                body = body.push(
                    action("Ayarları kaydet", msg(RebarMsg::SaveSettings), false).width(Fill),
                );
            }
            body = body.push(heading(
                "Çizimden demir alımı".into(),
                !ui.closed.contains(&Section::Read),
                msg(RebarMsg::Toggle(Section::Read)),
            ));
            if !ui.closed.contains(&Section::Read) {
                body = body
                    .push(
                        pick_list(Some(ui.mode), RebarMode::ALL, |m: &RebarMode| {
                            m.label().to_owned()
                        })
                        .on_select(|m| send(msg(RebarMsg::Mode(m))))
                        .text_size(11)
                        .padding([4, 6])
                        .width(Fill),
                    )
                    .push(text(ui.mode.hint()).size(10).style(muted_style))
                    .push(
                        row![
                            text("x adet").size(11).width(72),
                            field("1", &ui.multiplier, RebarMsg::Multiplier)
                        ]
                        .spacing(4)
                        .align_y(iced::Center),
                    );
            }
            body = body.push(
                pick_list(Some(ui.deduction), [false, true], |d: &bool| {
                    if *d { "− Minha" } else { "+ Ekle" }.to_owned()
                })
                .on_select(|v| send(msg(RebarMsg::Deduction(v))))
                .text_size(11)
                .padding([4, 6])
                .width(Fill),
            );
            if !ui.closed.contains(&Section::Read) {
                body = body
                    .push(action("Demir metrajı al", msg(RebarMsg::Read), true).width(Fill))
                    .push(
                        text(format!(
                            "{} · TEXT/MTEXT seçin, Enter ile tamamlayın.",
                            ui.mode.command()
                        ))
                        .size(10)
                        .style(muted_style),
                    );
            }
            body = body.push(heading(
                if ui.selected.is_some() {
                    "Seçili kaydı düzenle"
                } else {
                    "Manuel demir girişi"
                }
                .into(),
                !ui.closed.contains(&Section::Manual),
                msg(RebarMsg::Toggle(Section::Manual)),
            ));
            if !ui.closed.contains(&Section::Manual) {
                for (label, hint, value, change) in [
                    (
                        "Adet",
                        "1",
                        &ui.quantity,
                        RebarMsg::Quantity as fn(String) -> RebarMsg,
                    ),
                    ("Çap (mm)", "16", &ui.diameter, RebarMsg::Diameter),
                    ("Boy (cm)", "250", &ui.length, RebarMsg::Length),
                ] {
                    body = body.push(
                        row![text(label).size(11).width(72), field(hint, value, change)]
                            .spacing(4)
                            .align_y(iced::Center),
                    );
                }
                body = body
                    .push(action("Yeni kayıt ekle", msg(RebarMsg::AddManual), true).width(Fill));
                if ui.selected.is_some() {
                    body = body.push(
                        row![
                            action("Güncelle", msg(RebarMsg::Update), true),
                            action("Sil", msg(RebarMsg::Remove), false),
                            action("Seçimi kaldır", msg(RebarMsg::ClearEdit), false)
                        ]
                        .spacing(4),
                    );
                }
            }
            body = body.push(heading(
                format!("Demir kayıtları ({})", ui.rows.len()),
                !ui.closed.contains(&Section::Rows),
                msg(RebarMsg::Toggle(Section::Rows)),
            ));
            if !ui.closed.contains(&Section::Rows) {
                body = body.push(field(
                    "Çap / kaynak / handle ara…",
                    &ui.filter,
                    RebarMsg::Filter,
                ));
                let filter = ui.filter.trim().to_lowercase();
                for entry in ui.rows.iter().filter(|e| {
                    format!("{} {} {}", e.diameter_mm, e.source, e.handles)
                        .to_lowercase()
                        .contains(&filter)
                }) {
                    body = body.push(choice(
                        format!(
                            "{} {} × Ø{} · {:.2} cm",
                            if entry.deduction { "−" } else { "+" },
                            entry.quantity,
                            entry.diameter_mm,
                            entry.length_cm
                        ),
                        format!("{:.3} kg", entry.weight_kg()),
                        ui.selected == Some(entry.id),
                        msg(RebarMsg::Select(entry.id)),
                    ));
                    body = body.push(
                        text(format!(
                            "{:.3} m · {}",
                            entry.total_length_m(),
                            entry.source
                        ))
                        .size(10)
                        .style(muted_style),
                    );
                }
            }
            body = body.push(heading(
                "Demir icmali · çapa göre".into(),
                !ui.closed.contains(&Section::Summary),
                msg(RebarMsg::Toggle(Section::Summary)),
            ));
            if !ui.closed.contains(&Section::Summary) {
                let summary = summarize_rebar(&ui.rows);
                for row in &summary {
                    body = body
                        .push(
                            row![
                                text(format!("Ø{}", row.diameter_mm)).size(11).width(42),
                                text(format!("{:.3} m", row.length_m)).size(11).width(Fill),
                                text(format!("{:.3} kg", row.net_kg())).size(11)
                            ]
                            .spacing(4),
                        )
                        .push(
                            text(format!(
                                "Ekle {:.3} · Minha {:.3} kg · {:.6} ton",
                                row.added_kg,
                                row.deducted_kg,
                                row.net_kg() / 1000.
                            ))
                            .size(10)
                            .style(muted_style),
                        );
                }
                let total: f64 = summary.iter().map(|r| r.net_kg()).sum();
                body = body.push(
                    text(format!(
                        "Toplam: {:.3} kg · {:.6} ton",
                        total,
                        total / 1000.
                    ))
                    .size(11),
                );
            }
            body = body
                .push(action("Excel'e aktar (.xlsx)", msg(RebarMsg::Export), false).width(Fill));
        } else {
            body = body.push(text("Önce Metraj Projesi panelinden proje seçin.").size(11));
        }
        body = body.push(text(&self.status).size(10));
        crate::ui::dock::frame(
            crate::ui::dock::title_bar(PanelId::MetrajRebar, "Demir Metrajı".into(), chrome),
            scrollable(body),
            width,
        )
    }
}

impl OpenCADStudio {
    pub(super) fn on_rebar(&mut self, message: RebarMsg) -> Task<Message> {
        let saved_settings = matches!(&message, RebarMsg::SaveSettings);
        if matches!(message, RebarMsg::Read) {
            return self.dispatch_command(self.metraj.rebar.mode.command());
        }
        let result = (|| -> Result<(), String> {
            let project = self
                .metraj
                .project
                .as_ref()
                .ok_or("Önce metraj projesi seçin.")?;
            let ui = &mut self.metraj.rebar;
            match message {
                RebarMsg::Marking(value) => ui.marking = value,
                RebarMsg::Color(value) => ui.color = value,
                RebarMsg::RecolorText(value) => ui.recolor_text = value,
                RebarMsg::DrawBoxes(value) => ui.draw_boxes = value,
                RebarMsg::SaveSettings => project
                    .save_rebar_settings(&ui.settings()?)
                    .map_err(|e| e.to_string())?,
                RebarMsg::Mode(mode) => {
                    ui.mode = mode;
                    project
                        .save_rebar_settings(&ui.settings()?)
                        .map_err(|e| e.to_string())?;
                }
                RebarMsg::Multiplier(v) => ui.multiplier = v,
                RebarMsg::Tolerance(v) => ui.tolerance = v,
                RebarMsg::Symbols(v) => ui.symbols = v,
                RebarMsg::Deduction(v) => ui.deduction = v,
                RebarMsg::Quantity(v) => ui.quantity = v,
                RebarMsg::Diameter(v) => ui.diameter = v,
                RebarMsg::Length(v) => ui.length = v,
                RebarMsg::Filter(v) => ui.filter = v,
                RebarMsg::Toggle(section) => {
                    if !ui.closed.remove(&section) {
                        ui.closed.insert(section);
                    }
                }
                RebarMsg::ClearEdit => ui.selected = None,
                RebarMsg::Read => unreachable!(),
                RebarMsg::AddManual => {
                    project
                        .add_rebar(&[ui.manual()?])
                        .map_err(|e| e.to_string())?;
                    ui.reload(project)?;
                }
                RebarMsg::Select(id) => {
                    let entry = ui
                        .rows
                        .iter()
                        .find(|e| e.id == id)
                        .ok_or("Kayıt bulunamadı.")?;
                    ui.quantity = entry.quantity.to_string();
                    ui.diameter = entry.diameter_mm.to_string();
                    ui.length = entry.length_cm.to_string();
                    ui.deduction = entry.deduction;
                    ui.selected = Some(id);
                    ui.closed.remove(&Section::Manual);
                }
                RebarMsg::Update => {
                    project
                        .update_rebar(ui.selected.ok_or("Kayıt seçin.")?, &ui.manual()?)
                        .map_err(|e| e.to_string())?;
                    ui.reload(project)?;
                }
                RebarMsg::Remove => {
                    project
                        .remove_rebar(ui.selected.ok_or("Kayıt seçin.")?)
                        .map_err(|e| e.to_string())?;
                    ui.selected = None;
                    ui.reload(project)?;
                }
                RebarMsg::Export => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Excel", &["xlsx"])
                        .set_file_name("demir-metraj.xlsx")
                        .save_file()
                    {
                        crate::metraj::export_rebar(
                            &path,
                            &project.rebar_rows().map_err(|e| e.to_string())?,
                        )?;
                    }
                }
            }
            Ok(())
        })();
        self.metraj.status = match result {
            Err(error) => error,
            Ok(()) if saved_settings => "Demir ayarları kaydedildi.".into(),
            Ok(()) => String::new(),
        };
        Task::none()
    }
    pub(super) fn dispatch_rebar(&mut self, command: &str) -> Option<Task<Message>> {
        if command == "DEMIR" {
            self.tabs[self.active_tab].last_cmd = Some(command.into());
            self.dock_open_at_default(PanelId::MetrajRebar);
            return Some(self.dock_set_open(PanelId::MetrajRebar, true));
        }
        let mode = RebarMode::ALL
            .into_iter()
            .find(|m| m.command() == command)?;
        self.tabs[self.active_tab].last_cmd = Some(command.into());
        self.metraj.rebar.mode = mode;
        if let Err(error) = self.prepare_rebar() {
            self.metraj.status = error.clone();
            self.command_line.push_error(&error);
            return Some(Task::none());
        }
        let handles = self.tabs[self.active_tab].scene.selected_handles_in_order();
        if handles.is_empty() {
            let selection = crate::modules::draw::select::SelectObjectsCommand::new(command);
            self.command_line
                .push_info(&crate::command::CadCommand::prompt(&selection));
            self.tabs[self.active_tab].active_cmd = Some(Box::new(selection));
            return Some(self.finish_dispatch(command));
        }
        match self.read_selected_rebar(&handles) {
            Ok(status) => {
                self.metraj.status = status.clone();
                self.command_line.push_info(&status);
            }
            Err(error) => {
                self.metraj.status = error.clone();
                self.command_line.push_error(&error);
            }
        }
        Some(Task::none())
    }
    fn prepare_rebar(&self) -> Result<(), String> {
        let project = self
            .metraj
            .project
            .as_ref()
            .ok_or("Önce metraj projesi seçin; mahal gerekli değil.")?;
        project
            .save_rebar_settings(&self.metraj.rebar.settings()?)
            .map_err(|e| e.to_string())
    }
    fn read_selected_rebar(&mut self, handles: &[codec::Handle]) -> Result<String, String> {
        let i = self.active_tab;
        let project = self.metraj.project.as_ref().ok_or("Proje seçin.")?;
        let tab = &self.tabs[i];
        let texts: Vec<_> = handles
            .iter()
            .filter_map(|&handle| {
                let entity = tab.scene.document.get_entity(handle)?;
                let (value, position) = match entity {
                    codec::EntityType::Text(t) => (
                        crate::entities::text_support::resolve_dxf_special_chars(&t.value),
                        t.insertion_point,
                    ),
                    codec::EntityType::MText(t) => (
                        codec::entities::mtext_format::parse_mtext(&t.value, true).to_plain_text(),
                        t.insertion_point,
                    ),
                    _ => return None,
                };
                Some(RebarText {
                    handle: format!("{:X}", handle.value()),
                    text: value,
                    position: [position.x, position.y, position.z],
                    layer: entity.common().layer.clone(),
                })
            })
            .collect();
        let result = read_rebar(
            &texts,
            &self.metraj.rebar.settings()?,
            self.metraj.rebar.deduction,
            &drawing_key(tab.current_path.as_deref(), tab.id),
        )?;
        let processed: Vec<_> = result
            .processed
            .iter()
            .map(|s| {
                u64::from_str_radix(s, 16)
                    .map(codec::Handle::new)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<_, _>>()?;
        let settings = self.metraj.rebar.settings()?;
        let changes_drawing = settings.recolor_text || settings.draw_boxes;
        if (settings.recolor_text
            && processed
                .iter()
                .any(|h| tab.scene.is_layer_locked(*h) || tab.scene.is_reference_only(*h)))
            || (changes_drawing
                && tab
                    .scene
                    .document
                    .layers
                    .get("HESAPLANDI")
                    .is_some_and(|layer| layer.is_locked()))
        {
            return Err("Kaynak yazı veya HESAPLANDI katmanı kilitli; kayıt oluşturulmadı.".into());
        }
        let colors = super::rebar_marking::match_colors(
            settings.marking,
            settings.color,
            result.rows.len(),
            self.metraj.rebar.rows.len(),
        );
        let frames = if settings.draw_boxes {
            result
                .rows
                .iter()
                .zip(&colors)
                .map(|(r, &color)| {
                    super::rebar_marking::enclosing_box(&tab.scene, &r.handles, color)
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        project.add_rebar(&result.rows).map_err(|e| e.to_string())?;
        self.metraj.rebar.reload(project)?;
        if changes_drawing {
            self.push_undo_snapshot(i, "Demir metrajı işaretle");
            self.tabs[i].scene.ensure_layer("HESAPLANDI");
        }
        if settings.recolor_text {
            for (row, &color) in result.rows.iter().zip(&colors) {
                for hex in row.handles.split('|') {
                    let handle = codec::Handle::new(
                        u64::from_str_radix(hex, 16).expect("validated source handle"),
                    );
                    if let Some(entity) = self.tabs[i].scene.document.get_entity(handle) {
                        let entity = super::rebar_marking::recolor(entity, color);
                        self.tabs[i].scene.update_entity(entity);
                    }
                }
            }
        }
        let frame_count = frames.len();
        for frame in frames {
            self.tabs[i].scene.add_entity(frame);
        }
        self.tabs[i].scene.deselect_all();
        self.tabs[i].dirty |= changes_drawing;
        self.refresh_properties();
        Ok(format!(
            "{} demir kaydı · {} eşleşen yazı · {} kutu · {} eşleşmedi · {} yazı işlenmedi.",
            result.rows.len(),
            processed.len(),
            frame_count,
            result.unmatched,
            texts.len() - processed.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fresh() -> OpenCADStudio {
        let mut app = OpenCADStudio::new_for_test();
        app.automation_op(r#"{"op":"new"}"#);
        app.metraj.project = Some(Project::open(":memory:", "Demir").unwrap());
        app
    }
    fn add_text(
        app: &mut OpenCADStudio,
        value: &str,
        position: [f64; 3],
        mtext: bool,
    ) -> codec::Handle {
        let position = codec::types::Vector3::new(position[0], position[1], position[2]);
        let entity = if mtext {
            codec::EntityType::MText(Box::new(codec::entities::MText::with_value(
                value, position,
            )))
        } else {
            codec::EntityType::Text(
                codec::entities::Text::with_value(value, position).with_height(3.),
            )
        };
        app.tabs[app.active_tab].scene.add_entity(entity)
    }
    #[test]
    fn each_rebar_command_reads_without_a_room_marks_texts_and_repeats() {
        for mode in RebarMode::ALL {
            let mut app = fresh();
            let mut handles = Vec::new();
            match mode {
                RebarMode::Mod1 => {
                    handles.push(add_text(&mut app, "{\\C1;12%%c16}\\PL=250", [0.; 3], true))
                }
                RebarMode::Mod2 | RebarMode::Horizontal | RebarMode::Vertical => {
                    handles.push(add_text(&mut app, "12Ø16", [0.; 3], false));
                    let position = if mode == RebarMode::Vertical {
                        [0.005, 10., 0.]
                    } else {
                        [10., 0.005, 0.]
                    };
                    handles.push(add_text(&mut app, "L=250", position, false));
                }
                RebarMode::Quantityless => {
                    handles.push(add_text(&mut app, "Ø16", [0.; 3], false));
                    handles.push(add_text(&mut app, "L=250", [10.; 3], false));
                    handles.push(add_text(&mut app, "12 Adet", [20.; 3], false));
                }
            }
            app.tabs[app.active_tab].scene.select_entities(&handles);
            let _ = app.dispatch_command(mode.command());
            let project = app.metraj.project.as_ref().unwrap();
            let rows = project.rebar_rows().unwrap();
            assert_eq!(rows.len(), 1, "{}: {}", mode.command(), app.metraj.status);
            assert_eq!(
                (rows[0].quantity, rows[0].diameter_mm, rows[0].length_cm),
                (12, 16, 250.)
            );
            assert!(project.groups().unwrap().is_empty());
            assert!(app.metraj.selected_room.is_none());
            for &handle in &handles {
                assert_eq!(
                    app.tabs[app.active_tab]
                        .scene
                        .document
                        .get_entity(handle)
                        .unwrap()
                        .common()
                        .layer,
                    "HESAPLANDI"
                );
            }
            assert!(app.tabs[app.active_tab].scene.selected.is_empty());
            for repeat in [
                Message::CommandFinalize,
                Message::CommandSpace,
                Message::CommandSubmit,
            ] {
                let _ = app.update(repeat);
                assert!(app.tabs[app.active_tab].active_cmd.is_some());
                assert_eq!(
                    app.tabs[app.active_tab].last_cmd.as_deref(),
                    Some(mode.command())
                );
                let _ = app.update(Message::CommandEscape);
            }
            assert_eq!(
                app.metraj
                    .project
                    .as_ref()
                    .unwrap()
                    .rebar_rows()
                    .unwrap()
                    .len(),
                1
            );
            let _ = app.on_rebar(RebarMsg::Read);
            assert_eq!(
                app.tabs[app.active_tab].last_cmd.as_deref(),
                Some(mode.command())
            );
            assert!(app.tabs[app.active_tab].active_cmd.is_some());
        }
    }
    #[test]
    fn manual_panel_requires_explicit_add_and_updates_and_deletes_selected_rows() {
        let mut app = fresh();
        let _ = app.on_rebar(RebarMsg::AddManual);
        let id = app.metraj.project.as_ref().unwrap().rebar_rows().unwrap()[0].id;
        let _ = app.on_rebar(RebarMsg::Select(id));
        assert_eq!(
            app.metraj
                .project
                .as_ref()
                .unwrap()
                .rebar_rows()
                .unwrap()
                .len(),
            1
        );
        let _ = app.on_rebar(RebarMsg::Quantity("2".into()));
        let _ = app.on_rebar(RebarMsg::Update);
        assert_eq!(
            app.metraj.project.as_ref().unwrap().rebar_rows().unwrap()[0].quantity,
            2
        );
        let _ = app.on_rebar(RebarMsg::Deduction(true));
        let _ = app.on_rebar(RebarMsg::AddManual);
        assert!(
            summarize_rebar(&app.metraj.project.as_ref().unwrap().rebar_rows().unwrap())[0]
                .net_kg()
                .abs()
                < 1e-10
        );
        let _ = app.on_rebar(RebarMsg::Remove);
        let rows = app.metraj.project.as_ref().unwrap().rebar_rows().unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].deduction);
    }
    #[test]
    fn marking_modes_apply_per_match_and_leave_unmatched_texts_unchanged() {
        for marking in RebarMarking::ALL {
            for boxes in [false, true] {
                for recolor in [false, true] {
                    let mut app = fresh();
                    let _ = app.on_rebar(RebarMsg::Marking(marking));
                    let _ = app.on_rebar(RebarMsg::Color(6));
                    let _ = app.on_rebar(RebarMsg::DrawBoxes(boxes));
                    let _ = app.on_rebar(RebarMsg::RecolorText(recolor));
                    let _ = app.on_rebar(RebarMsg::SaveSettings);
                    let settings = app
                        .metraj
                        .project
                        .as_ref()
                        .unwrap()
                        .rebar_settings()
                        .unwrap();
                    assert_eq!(settings.marking, marking);
                    assert_eq!(settings.draw_boxes, boxes);
                    assert_eq!(settings.recolor_text, recolor);
                    let cap = add_text(&mut app, r"{\C1;8Ø20/20}", [0.; 3], true);
                    let mut entity = app.tabs[app.active_tab]
                        .scene
                        .document
                        .get_entity(cap)
                        .unwrap()
                        .clone();
                    if let codec::EntityType::MText(t) = &mut entity {
                        t.rectangle_width = 10000.;
                    }
                    app.tabs[app.active_tab].scene.update_entity(entity);
                    let length = add_text(&mut app, "L=200", [30., 0., 0.], false);
                    let second = add_text(&mut app, "7Ø12/12", [200., 0., 0.], false);
                    let second_length = add_text(&mut app, "L=300", [230., 0., 0.], false);
                    let unused = add_text(&mut app, "AÇIKLAMA", [1000., 1000., 0.], false);
                    let handles = [cap, length, second, second_length, unused];
                    let before: Vec<_> = handles
                        .iter()
                        .map(|h| {
                            let e = app.tabs[app.active_tab]
                                .scene
                                .document
                                .get_entity(*h)
                                .unwrap();
                            (e.common().layer.clone(), e.common().color)
                        })
                        .collect();
                    let dirty_before = app.tabs[app.active_tab].dirty;
                    app.tabs[app.active_tab].scene.select_entities(&handles);
                    let _ = app.dispatch_command("DM2");
                    let rows = app.metraj.project.as_ref().unwrap().rebar_rows().unwrap();
                    assert_eq!(rows.len(), 2, "{}", app.metraj.status);
                    assert_eq!((rows[0].quantity, rows[1].quantity), (8, 7));
                    let frames: Vec<_> = app.tabs[app.active_tab]
                        .scene
                        .document
                        .entities()
                        .filter_map(|e| {
                            if let codec::EntityType::LwPolyline(p) = e {
                                Some(p)
                            } else {
                                None
                            }
                        })
                        .collect();
                    assert_eq!(frames.len(), if boxes { 2 } else { 0 });
                    for frame in frames {
                        assert!(frame.is_closed);
                        assert_eq!(frame.vertices.len(), 4);
                        let lo = frame
                            .vertices
                            .iter()
                            .map(|v| v.location.x)
                            .fold(f64::INFINITY, f64::min);
                        let hi = frame
                            .vertices
                            .iter()
                            .map(|v| v.location.x)
                            .fold(f64::NEG_INFINITY, f64::max);
                        assert!(
                            hi - lo < 100.,
                            "Box used editing width or whole selection: {}",
                            hi - lo
                        );
                        let color = if marking == RebarMarking::Color {
                            6
                        } else if (hi + lo) * 0.5 < 100. {
                            1
                        } else {
                            2
                        };
                        assert_eq!(frame.common.color, codec::types::Color::Index(color));
                    }
                    for (i, h) in handles.iter().enumerate() {
                        let e = app.tabs[app.active_tab]
                            .scene
                            .document
                            .get_entity(*h)
                            .unwrap();
                        if recolor && *h != unused {
                            let color = if marking == RebarMarking::Color {
                                6
                            } else if i < 2 {
                                1
                            } else {
                                2
                            };
                            assert_eq!(e.common().layer, "HESAPLANDI");
                            assert_eq!(e.common().color, codec::types::Color::Index(color));
                        } else {
                            assert_eq!((e.common().layer.clone(), e.common().color), before[i]);
                        }
                    }
                    if let codec::EntityType::MText(t) = app.tabs[app.active_tab]
                        .scene
                        .document
                        .get_entity(cap)
                        .unwrap()
                    {
                        assert_eq!(
                            t.value,
                            if recolor && marking == RebarMarking::Color {
                                r"{\C6;8Ø20/20}"
                            } else {
                                r"{\C1;8Ø20/20}"
                            }
                        );
                    }
                    if !recolor && !boxes {
                        assert_eq!(app.tabs[app.active_tab].dirty, dirty_before);
                        assert!(!app.tabs[app.active_tab]
                            .scene
                            .document
                            .layers
                            .contains("HESAPLANDI"));
                    }
                }
            }
        }
    }
    #[test]
    fn rotated_text_box_uses_ink_and_nonplanar_batch_does_not_save_partial_rows() {
        let mut app = fresh();
        let handle = add_text(&mut app, "12Ø16 L=250", [1e9, 1e9, 0.], false);
        let mut entity = app.tabs[app.active_tab]
            .scene
            .document
            .get_entity(handle)
            .unwrap()
            .clone();
        if let codec::EntityType::Text(t) = &mut entity {
            t.rotation = std::f64::consts::FRAC_PI_4;
        }
        app.tabs[app.active_tab].scene.update_entity(entity);
        let frame = super::super::rebar_marking::enclosing_box(
            &app.tabs[app.active_tab].scene,
            &format!("{:X}", handle.value()),
            3,
        )
        .unwrap();
        let codec::EntityType::LwPolyline(frame) = frame else {
            panic!("No frame")
        };
        let p: Vec<_> = frame
            .vertices
            .iter()
            .map(|v| glam::DVec2::new(v.location.x, v.location.y))
            .collect();
        let area = (p[1] - p[0]).length() * (p[3] - p[0]).length();
        let lo = p
            .iter()
            .copied()
            .fold(glam::DVec2::splat(f64::INFINITY), |a, b| a.min(b));
        let hi = p
            .iter()
            .copied()
            .fold(glam::DVec2::splat(f64::NEG_INFINITY), |a, b| a.max(b));
        assert!(area < (hi - lo).x * (hi - lo).y * 0.5);
        let _ = app.on_rebar(RebarMsg::Marking(RebarMarking::Mixed));
        let _ = app.on_rebar(RebarMsg::DrawBoxes(true));
        let first = add_text(&mut app, "4Ø12", [0.; 3], false);
        let first_length = add_text(&mut app, "L=200", [10., 0., 0.], false);
        let second = add_text(&mut app, "2Ø8", [200., 0., 0.], false);
        let second_length = add_text(&mut app, "L=100", [210., 0., 10.], false);
        app.tabs[app.active_tab].scene.select_entities(&[
            first,
            first_length,
            second,
            second_length,
        ]);
        let _ = app.dispatch_command("DM2");
        assert!(app
            .metraj
            .project
            .as_ref()
            .unwrap()
            .rebar_rows()
            .unwrap()
            .is_empty());
        assert!(app.metraj.status.contains("düzlem"));
        for h in [first, first_length, second, second_length] {
            assert_eq!(
                app.tabs[app.active_tab]
                    .scene
                    .document
                    .get_entity(h)
                    .unwrap()
                    .common()
                    .layer,
                "0"
            );
        }
    }
}
