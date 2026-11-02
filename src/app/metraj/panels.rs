use super::{
    section_label, send, unit_label, AmountMode, AssignmentPanelSection, Chrome, Message,
    MetrajMsg, MetrajUi, NamedRow, PanelId, Project, RoomSection, Unit,
};
use crate::ui::style::common::muted_style;
use crate::ui::style::form::{button_style, field_style};
use iced::widget::{
    button, column, container, mouse_area, pick_list, row, scrollable, text, text_input, tooltip,
    Space,
};
use iced::{Background, Border, Element, Fill, Theme};

const FONT: f32 = 11.0;
const SMALL: f32 = 10.0;
const ROW: f32 = 24.0;

fn drag_handle(message: MetrajMsg) -> Element<'static, Message> {
    mouse_area(
        container(text("⠿").size(FONT))
            .padding(4)
            .width(22)
            .height(ROW),
    )
    .on_press(send(message))
    .on_release(send(MetrajMsg::CancelDrag))
    .interaction(iced::mouse::Interaction::Grab)
    .into()
}

fn input<'a>(
    hint: &'a str,
    value: &'a str,
    message: fn(String) -> MetrajMsg,
) -> Element<'a, Message> {
    text_input(hint, value)
        .on_input(move |value| send(message(value)))
        .size(FONT)
        .padding([4, 6])
        .style(field_style)
        .into()
}

pub(super) fn action<'a>(
    label: &'a str,
    message: MetrajMsg,
    accent: bool,
) -> button::Button<'a, Message> {
    button(text(label).size(FONT))
        .on_press(send(message))
        .padding([4, 6])
        .height(ROW)
        .style(button_style(accent))
}

fn row_style(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.palette();
        let pair = if selected {
            palette.primary.weak
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            palette.background.strong
        } else {
            palette.background.base
        };
        button::Style {
            background: Some(Background::Color(pair.color)),
            text_color: pair.text,
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub(super) fn choice<'a>(
    label: String,
    detail: String,
    selected: bool,
    message: MetrajMsg,
) -> Element<'a, Message> {
    let hint = format!("{label}  {detail}");
    let button = button(
        row![
            text(label)
                .size(FONT)
                .width(Fill)
                .wrapping(text::Wrapping::None),
            text(detail).size(SMALL).style(muted_style)
        ]
        .spacing(5)
        .align_y(iced::Center),
    )
    .on_press(send(message))
    .width(Fill)
    .height(ROW)
    .padding([3, 5])
    .style(row_style(selected));
    tooltip(button, text(hint).size(FONT), tooltip::Position::Bottom).into()
}

pub(super) fn heading<'a>(label: String, open: bool, message: MetrajMsg) -> Element<'a, Message> {
    button(
        row![
            text(if open { "▾" } else { "▸" }).size(FONT),
            text(label).size(FONT)
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .on_press(send(message))
    .height(ROW)
    .padding([4, 6])
    .style(button_style(false))
    .width(Fill)
    .into()
}

fn card<'a>(content: impl Into<Element<'a, Message>>, selected: bool) -> Element<'a, Message> {
    container(content)
        .padding(5)
        .width(Fill)
        .style(move |theme: &Theme| {
            let palette = theme.palette();
            container::Style {
                background: Some(Background::Color(if selected {
                    palette.primary.weak.color
                } else {
                    palette.background.weak.color
                })),
                border: Border {
                    color: if selected {
                        palette.primary.base.color
                    } else {
                        palette.background.neutral.color
                    },
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn tree_row<'a>(
    label: String,
    detail: String,
    depth: u16,
    selected: bool,
    expanded: bool,
    toggle: Option<MetrajMsg>,
    select: MetrajMsg,
) -> Element<'a, Message> {
    let arrow: Element<'a, Message> = match toggle {
        Some(message) => action(if expanded { "▾" } else { "▸" }, message, false)
            .width(18)
            .padding(0)
            .style(button::text)
            .into(),
        None => Space::new().width(18).into(),
    };
    row![
        Space::new().width(depth as f32 * 10.0),
        arrow,
        choice(label, detail, selected, select)
    ]
    .spacing(1)
    .align_y(iced::Center)
    .into()
}

fn matches(value: &str, query: &str) -> bool {
    value.to_lowercase().contains(&query.trim().to_lowercase())
}

impl MetrajUi {
    fn room_context(&self) -> Option<(String, String, String)> {
        let project = self.project.as_ref()?;
        let room = self.selected_room?;
        let (floor_id, group_id) = project.room_location(room).ok()??;
        let group = project
            .groups()
            .ok()?
            .into_iter()
            .find(|row| row.id == group_id)?;
        let floor = project
            .floors(group_id)
            .ok()?
            .into_iter()
            .find(|row| row.id == floor_id)?;
        let name = project
            .rooms(floor_id)
            .ok()?
            .into_iter()
            .find(|row| row.id == room)?
            .name;
        let info = project.room_info(room).ok()??;
        Some((
            name,
            format!(
                "{} / {} / {}",
                project.name().unwrap_or_default(),
                group.name,
                floor.name
            ),
            format!(
                "{:.2} m²  ·  {:.2} m{}",
                info.measurement.area_m2,
                info.measurement.perimeter_m,
                if !info.bound {
                    "  ·  Sınır yok"
                } else if !info.boundary_valid {
                    "  ·  Sınır geçersiz"
                } else {
                    ""
                }
            ),
        ))
    }

    fn target_view(&self) -> Element<'_, Message> {
        let rooms = self.target_rooms();
        if rooms.len() > 1 {
            let names: Vec<_> = rooms
                .iter()
                .take(3)
                .filter_map(|id| {
                    let p = self.project.as_ref()?;
                    let (floor, _) = p.room_location(*id).ok()??;
                    p.rooms(floor)
                        .ok()?
                        .into_iter()
                        .find(|r| r.id == *id)
                        .map(|r| r.name)
                })
                .collect();
            return card(
                column![
                    text(format!("{} mahal seçili", rooms.len())).size(FONT),
                    text(format!(
                        "{}{}",
                        names.join(", "),
                        if rooms.len() > 3 {
                            format!(" ve {} mahal daha", rooms.len() - 3)
                        } else {
                            String::new()
                        }
                    ))
                    .size(SMALL)
                    .style(muted_style),
                    text("Uygula, her mahalin kendi ölçüsünü kullanır.")
                        .size(SMALL)
                        .style(muted_style),
                ]
                .spacing(2),
                true,
            );
        }
        match self.room_context() {
            Some((name, path, measurement)) => card(
                column![
                    text(format!("Hedef mahal: {name}")).size(FONT),
                    text(path).size(SMALL).style(muted_style),
                    text(measurement).size(SMALL).style(muted_style),
                ]
                .spacing(2),
                true,
            ),
            None => card(
                text("Hedef mahal seçilmedi. Ağaçtan veya çizimden mahal seçin.").size(FONT),
                false,
            ),
        }
    }

    pub(crate) fn tree_view(&self, width: f32, chrome: Chrome) -> Element<'_, Message> {
        let selected = self
            .project_entries
            .iter()
            .find(|entry| self.project.as_ref().map(Project::id) == Some(entry.id))
            .cloned();
        let project_picker = pick_list(
            selected,
            self.project_entries.clone(),
            |entry: &NamedRow| entry.name.clone(),
        )
        .placeholder("Proje seçin")
        .on_select(|entry| send(MetrajMsg::SelectProject(entry.id)))
        .text_size(FONT)
        .padding([4, 6])
        .width(Fill);
        let mut body = column![heading(
            "Proje ve mahal düzeni".into(),
            self.tree_form_open,
            MetrajMsg::ToggleTreeForm
        )]
        .spacing(3);
        if self.tree_form_open {
            let mut form = column![
                input("Yeni proje adı", &self.project_name, MetrajMsg::ProjectName),
                row![
                    action("Proje oluştur", MetrajMsg::NewProject, true),
                    action("İçe aktar", MetrajMsg::OpenProject, false)
                ]
                .spacing(4),
            ]
            .spacing(4);
            if self.project.is_some() {
                form = form
                    .push(
                        row![
                            input(
                                "Kat yüksekliği (m)",
                                &self.project_height,
                                MetrajMsg::ProjectHeight
                            ),
                            action("Kaydet", MetrajMsg::SaveProjectHeight, false)
                        ]
                        .spacing(4),
                    )
                    .push(
                        row![
                            input("İş grubu adı", &self.group_name, MetrajMsg::GroupName),
                            action("Ekle", MetrajMsg::AddGroup, false)
                        ]
                        .spacing(4),
                    );
                if self.selected_group.is_some() {
                    form = form.push(
                        row![
                            input("Kat adı", &self.floor_name, MetrajMsg::FloorName),
                            action("Ekle", MetrajMsg::AddFloor, false)
                        ]
                        .spacing(4),
                    );
                }
                if self.selected_floor.is_some() {
                    form = form
                        .push(input("Mahal adı", &self.room_name, MetrajMsg::RoomName))
                        .push(
                            row![
                                action("Mahal ekle", MetrajMsg::AddRoom, false),
                                action("Adlandır", MetrajMsg::RenameRoom, false)
                            ]
                            .spacing(4),
                        );
                }
                form = form.push(
                    row![
                        action("MM · Mahal çiz", MetrajMsg::StartMm, false),
                        action("NM · Dönüştür", MetrajMsg::StartNm, false)
                    ]
                    .spacing(4),
                );
            }
            body = body.push(card(form, false));
        }
        if let Some(project) = &self.project {
            let searching = !self.room_filter.trim().is_empty();
            for group in project.groups().unwrap_or_default() {
                let group_match = matches(&group.name, &self.room_filter);
                let floors: Vec<_> = project
                    .floors(group.id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|floor| {
                        let parent_match = group_match || matches(&floor.name, &self.room_filter);
                        let rooms: Vec<_> = project
                            .rooms(floor.id)
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|room| {
                                !searching || parent_match || matches(&room.name, &self.room_filter)
                            })
                            .collect();
                        (!searching || parent_match || !rooms.is_empty()).then_some((floor, rooms))
                    })
                    .collect();
                if searching && floors.is_empty() && !group_match {
                    continue;
                }
                let open = searching || self.expanded_groups.contains(&group.id);
                body = body.push(tree_row(
                    group.name,
                    String::new(),
                    0,
                    self.selected_group == Some(group.id) && self.selected_floor.is_none(),
                    open,
                    Some(MetrajMsg::ToggleGroup(group.id)),
                    MetrajMsg::SelectGroup(group.id),
                ));
                if !open {
                    continue;
                }
                for (floor, rooms) in floors {
                    let open = searching || self.expanded_floors.contains(&floor.id);
                    body = body.push(tree_row(
                        floor.name,
                        format!("{} mahal", rooms.len()),
                        1,
                        self.selected_floor == Some(floor.id) && self.selected_room.is_none(),
                        open,
                        Some(MetrajMsg::ToggleFloor(floor.id)),
                        MetrajMsg::SelectFloor(floor.id),
                    ));
                    if !open {
                        continue;
                    }
                    for room in rooms {
                        let open = self.expanded_rooms.contains(&room.id);
                        let detail = project
                            .room_info(room.id)
                            .ok()
                            .flatten()
                            .map(|info| format!("{:.2} m²", info.measurement.area_m2))
                            .unwrap_or_default();
                        body = body.push(
                            mouse_area(tree_row(
                                room.name,
                                detail,
                                2,
                                self.is_target_room(room.id),
                                open,
                                Some(MetrajMsg::ToggleRoom(room.id)),
                                MetrajMsg::SelectRoom(room.id),
                            ))
                            .on_release(send(MetrajMsg::DropItem(room.id, None))),
                        );
                        if !open {
                            continue;
                        }
                        let assignments = project.room_assignments(room.id).unwrap_or_default();
                        for section in [
                            RoomSection::Wall,
                            RoomSection::Floor,
                            RoomSection::Ceiling,
                            RoomSection::Other,
                        ] {
                            let open = self.expanded_sections.contains(&(room.id, section));
                            let entries: Vec<_> = assignments
                                .iter()
                                .filter(|entry| entry.section == section)
                                .collect();
                            body = body.push(
                                mouse_area(tree_row(
                                    section_label(section).into(),
                                    entries.len().to_string(),
                                    3,
                                    self.is_target_room(room.id) && self.section == section,
                                    open,
                                    Some(MetrajMsg::ToggleSection(room.id, section)),
                                    MetrajMsg::SelectRoomSection(room.id, section),
                                ))
                                .on_release(send(MetrajMsg::DropItem(room.id, Some(section)))),
                            );
                            if open {
                                for entry in entries {
                                    body = body.push(tree_row(
                                        format!(
                                            "{} {}",
                                            if entry.is_deduction { "−" } else { "+" },
                                            entry.poz
                                        ),
                                        format!("{:.2} {}", entry.net, unit_label(entry.unit)),
                                        4,
                                        self.selected_assignment == Some(entry.id),
                                        false,
                                        None,
                                        MetrajMsg::SelectRoomAssignment(room.id, entry.id),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut layout = column![
            project_picker,
            input(
                "Mahal / kat / iş grubu ara…",
                &self.room_filter,
                MetrajMsg::RoomFilter
            ),
            scrollable(body).height(Fill),
            self.target_view()
        ]
        .spacing(4);
        if !self.status.is_empty() {
            layout = layout.push(text(&self.status).size(SMALL));
        }
        crate::ui::dock::frame(
            crate::ui::dock::title_bar(PanelId::MetrajTree, "Mahaller".into(), chrome),
            layout,
            width,
        )
    }

    fn item_creation_form(&self) -> Element<'_, Message> {
        let mut body = column![].spacing(3);
        body = body.push(heading(
            "Poz oluştur".into(),
            self.item_form_open,
            MetrajMsg::ToggleItemForm,
        ));
        if self.item_form_open {
            body = body.push(card(
                column![
                    input("Poz no", &self.item_poz, MetrajMsg::ItemPoz),
                    input("Kalem adı", &self.item_name, MetrajMsg::ItemName),
                    input(
                        "Kısa açıklama",
                        &self.item_short_description,
                        MetrajMsg::ItemShortDescription
                    ),
                    row![
                        pick_list(
                            Some(self.unit),
                            [Unit::SquareMeter, Unit::Meter, Unit::Each],
                            |unit: &Unit| unit_label(*unit).to_owned()
                        )
                        .on_select(|unit| send(MetrajMsg::Unit(unit)))
                        .text_size(FONT)
                        .padding([4, 6])
                        .width(Fill),
                        action("Oluştur", MetrajMsg::AddItem, true)
                    ]
                    .spacing(4),
                ]
                .spacing(4),
                false,
            ));
        }
        body.into()
    }

    pub(crate) fn items_view(&self, width: f32, chrome: Chrome) -> Element<'_, Message> {
        let tabs = row![
            action(
                "Kalemler",
                MetrajMsg::CatalogGroups(false),
                !self.catalog_groups
            )
            .width(Fill),
            action(
                "Grup pozları",
                MetrajMsg::CatalogGroups(true),
                self.catalog_groups
            )
            .width(Fill),
        ]
        .spacing(3);
        let filter = if self.catalog_groups {
            input(
                "Grup adı / içerdiği poz ile filtrele…",
                &self.group_filter,
                MetrajMsg::GroupFilter,
            )
        } else {
            input(
                "Poz no ile filtrele…",
                &self.poz_filter,
                MetrajMsg::PozFilter,
            )
        };
        let mut body = column![].spacing(3);
        if !self.catalog_groups {
            body = body.push(self.item_creation_form());
            if let Some(project) = &self.project {
                let mut items = project.items().unwrap_or_default();
                items.sort_by(|a, b| a.poz.cmp(&b.poz).then(a.name.cmp(&b.name)));
                let total = items.len();
                items.retain(|item| matches(&item.poz, &self.poz_filter));
                body = body.push(heading(
                    format!("Kalemler · {} / {total}", items.len()),
                    self.item_list_open,
                    MetrajMsg::ToggleItemList,
                ));
                if self.item_list_open {
                    for item in items {
                        let label = format!("{} · {}", item.poz, item.name);
                        let row = container(
                            row![
                                text(format!("⠿  {label}"))
                                    .size(FONT)
                                    .width(Fill)
                                    .wrapping(text::Wrapping::None),
                                text(unit_label(item.unit)).size(SMALL).style(muted_style),
                            ]
                            .spacing(5)
                            .align_y(iced::Center),
                        )
                        .height(ROW)
                        .padding([3, 5])
                        .width(Fill);
                        body = body.push(tooltip(
                            mouse_area(row)
                                .on_press(send(MetrajMsg::DragItem(item.id)))
                                .on_release(send(MetrajMsg::CancelDrag))
                                .interaction(iced::mouse::Interaction::Grab),
                            text(format!("{label}\n{}", item.short_description)).size(FONT),
                            tooltip::Position::Bottom,
                        ));
                    }
                }
            }
        } else {
            body = body.push(heading(
                "Grup pozu oluştur".into(),
                self.group_form_open,
                MetrajMsg::ToggleGroupForm,
            ));
            if self.group_form_open {
                body = body.push(card(
                    row![
                        input(
                            "Grup adı / poz no",
                            &self.quantity_group_name,
                            MetrajMsg::QuantityGroupName
                        ),
                        action("Oluştur", MetrajMsg::AddQuantityGroup, true)
                    ]
                    .spacing(4),
                    false,
                ));
            }
            if let Some(project) = &self.project {
                let groups = project.quantity_groups().unwrap_or_default();
                body = body.push(heading(
                    format!("Grup pozları · {}", groups.len()),
                    self.group_list_open,
                    MetrajMsg::ToggleGroupList,
                ));
                if self.group_list_open {
                    for group in groups {
                        let members = project.group_items(group.id).unwrap_or_default();
                        if !matches(&group.name, &self.group_filter)
                            && !members.iter().any(|item| {
                                matches(&item.poz, &self.group_filter)
                                    || matches(&item.name, &self.group_filter)
                            })
                        {
                            continue;
                        }
                        let open = self.expanded_quantity_groups.contains(&group.id);
                        body = body.push(
                            row![
                                mouse_area(container(text("⠿").size(FONT)).padding(4))
                                    .on_press(send(MetrajMsg::DragQuantityGroup(group.id)))
                                    .on_release(send(MetrajMsg::CancelDrag))
                                    .interaction(iced::mouse::Interaction::Grab),
                                choice(
                                    format!("{} {}", if open { "▾" } else { "▸" }, group.name),
                                    members.len().to_string(),
                                    self.selected_quantity_group == Some(group.id),
                                    MetrajMsg::ToggleQuantityGroup(group.id)
                                ),
                            ]
                            .spacing(2)
                            .align_y(iced::Center),
                        );
                        if !open {
                            continue;
                        }
                        for member in &members {
                            body = body.push(
                                row![
                                    Space::new().width(18),
                                    text(format!("{} · {}", member.poz, member.name))
                                        .size(FONT)
                                        .width(Fill),
                                    action(
                                        "×",
                                        MetrajMsg::RemoveItemFromQuantityGroup(group.id, member.id),
                                        false
                                    )
                                    .width(22)
                                ]
                                .spacing(3)
                                .align_y(iced::Center),
                            );
                        }
                        body = body.push(
                            action(
                                if self.selected_quantity_group == Some(group.id) {
                                    "Kalem eklemeyi kapat"
                                } else {
                                    "Gruba kalem ekle…"
                                },
                                MetrajMsg::SelectQuantityGroup(group.id),
                                false,
                            )
                            .width(Fill),
                        );
                        if self.selected_quantity_group == Some(group.id) {
                            let mut available = column![].spacing(1);
                            for item in
                                project
                                    .items()
                                    .unwrap_or_default()
                                    .into_iter()
                                    .filter(|item| {
                                        !members.iter().any(|member| member.id == item.id)
                                            && (matches(&item.poz, &self.member_filter)
                                                || matches(&item.name, &self.member_filter))
                                    })
                            {
                                available = available.push(choice(
                                    format!("+ {} · {}", item.poz, item.name),
                                    unit_label(item.unit).into(),
                                    false,
                                    MetrajMsg::AddItemToQuantityGroup(item.id),
                                ));
                            }
                            body = body.push(card(
                                column![
                                    input(
                                        "Eklenecek poz / kalem ara…",
                                        &self.member_filter,
                                        MetrajMsg::MemberFilter
                                    ),
                                    scrollable(available).height(120)
                                ]
                                .spacing(3),
                                false,
                            ));
                        }
                    }
                }
            }
        }
        if self.project.is_none() {
            body = body.push(text("Katalog için proje seçin.").size(FONT));
        }
        let mut layout = column![
            tabs,
            filter,
            scrollable(body).height(Fill),
            text("⠿ tutamacından mahal veya çizim üzerine sürükleyin.")
                .size(SMALL)
                .style(muted_style)
        ]
        .spacing(4);
        if !self.status.is_empty() {
            layout = layout.push(text(&self.status).size(SMALL));
        }
        crate::ui::dock::frame(
            crate::ui::dock::title_bar(PanelId::MetrajItems, "Metraj kalemleri".into(), chrome),
            layout,
            width,
        )
    }

    pub(crate) fn assignments_view(&self, width: f32, chrome: Chrome) -> Element<'_, Message> {
        let is_open = |section| !self.closed_assignment_sections.contains(&section);
        let mut body = column![
            self.item_creation_form(),
            heading(
                "Atama ayarları".into(),
                is_open(AssignmentPanelSection::Settings),
                MetrajMsg::ToggleAssignmentSection(AssignmentPanelSection::Settings)
            )
        ]
        .spacing(4);
        if is_open(AssignmentPanelSection::Settings) {
            let dropdown = pick_list(
                Some(self.section),
                [
                    RoomSection::Wall,
                    RoomSection::Floor,
                    RoomSection::Ceiling,
                    RoomSection::Other,
                ],
                |section: &RoomSection| section_label(*section).to_owned(),
            )
            .on_select(|section| send(MetrajMsg::Section(section)))
            .text_size(FONT)
            .padding([4, 6])
            .width(Fill);
            let mode = pick_list(
                Some(self.mode),
                [
                    AmountMode::Room,
                    AmountMode::PerimeterHeight,
                    AmountMode::LengthHeight,
                    AmountMode::Manual,
                ],
                |mode: &AmountMode| mode.label().to_owned(),
            )
            .on_select(|mode| send(MetrajMsg::Mode(mode)))
            .text_size(FONT)
            .padding([4, 6])
            .width(Fill);
            let kind = pick_list(
                Some(self.drag_deduction),
                [false, true],
                |deduction: &bool| if *deduction { "− Minha" } else { "+ Ekle" }.to_owned(),
            )
            .on_select(|value| send(MetrajMsg::DragDeduction(value)))
            .text_size(FONT)
            .padding([4, 6])
            .width(Fill);
            let mut form = column![
                row![text("Bölüm").size(FONT).width(68), dropdown]
                    .spacing(4)
                    .align_y(iced::Center),
                row![text("Hesap").size(FONT).width(68), mode]
                    .spacing(4)
                    .align_y(iced::Center),
                row![text("Tip").size(FONT).width(68), kind]
                    .spacing(4)
                    .align_y(iced::Center),
            ]
            .spacing(4);
            if self.mode == AmountMode::LengthHeight {
                form = form.push(input("Uzunluk (m)", &self.length, MetrajMsg::Length));
            }
            if matches!(
                self.mode,
                AmountMode::PerimeterHeight | AmountMode::LengthHeight
            ) {
                form = form.push(input("Yükseklik (m)", &self.height, MetrajMsg::Height));
            } else if self.mode == AmountMode::Manual {
                form = form.push(input("Miktar", &self.manual, MetrajMsg::Manual));
            } else {
                form = form.push(
                    text("m²: kapalı alan · m: çevre · adet: 1")
                        .size(SMALL)
                        .style(muted_style),
                );
            }
            body = body.push(card(form, false));
        }
        body = body.push(heading(
            "Kalem / grup seçimi".into(),
            is_open(AssignmentPanelSection::Catalog),
            MetrajMsg::ToggleAssignmentSection(AssignmentPanelSection::Catalog),
        ));
        if is_open(AssignmentPanelSection::Catalog) {
            body = body.push(input(
                "Poz / kalem / grup ara…",
                &self.assignment_filter,
                MetrajMsg::AssignmentFilter,
            ));
            let mut catalog = column![].spacing(1);
            if let Some(project) = &self.project {
                let mut items = project.items().unwrap_or_default();
                items.sort_by(|a, b| a.poz.cmp(&b.poz).then(a.name.cmp(&b.name)));
                for item in items.into_iter().filter(|item| {
                    matches(&item.poz, &self.assignment_filter)
                        || matches(&item.name, &self.assignment_filter)
                }) {
                    let selected = self.pending_items.contains(&item.id);
                    let entry = choice(
                        format!(
                            "{} {} · {}",
                            if selected { "☑" } else { "☐" },
                            item.poz,
                            item.name
                        ),
                        unit_label(item.unit).into(),
                        selected,
                        MetrajMsg::ToggleCatalogItem(item.id),
                    );
                    catalog = catalog
                        .push(row![drag_handle(MetrajMsg::DragItem(item.id)), entry].spacing(3));
                }
                for group in project.quantity_groups().unwrap_or_default() {
                    let members = project.group_items(group.id).unwrap_or_default();
                    if !matches(&group.name, &self.assignment_filter)
                        && !members
                            .iter()
                            .any(|item| matches(&item.poz, &self.assignment_filter))
                    {
                        continue;
                    }
                    let selected = self.pending_groups.contains(&group.id);
                    let entry = choice(
                        format!("{} Grup · {}", if selected { "☑" } else { "☐" }, group.name),
                        format!("{} kalem", members.len()),
                        selected,
                        MetrajMsg::ToggleCatalogGroup(group.id),
                    );
                    catalog = catalog.push(
                        row![drag_handle(MetrajMsg::DragQuantityGroup(group.id)), entry].spacing(3),
                    );
                }
            }
            body = body.push(card(scrollable(catalog).height(150), false));
        }
        body = body.push(heading(
            "Mahaldeki atamalar".into(),
            is_open(AssignmentPanelSection::Current),
            MetrajMsg::ToggleAssignmentSection(AssignmentPanelSection::Current),
        ));
        if is_open(AssignmentPanelSection::Current) {
            body = body.push(input(
                "Atanmış poz / kalem ara…",
                &self.assigned_filter,
                MetrajMsg::AssignedFilter,
            ));
            let mut current = column![].spacing(1);
            if let (Some(project), Some(_)) = (&self.project, self.selected_room) {
                let targets = self.target_rooms();
                let multiple = targets.len() > 1;
                for room in targets {
                    if multiple {
                        let name = project
                            .room_location(room)
                            .ok()
                            .flatten()
                            .and_then(|(floor, _)| project.rooms(floor).ok())
                            .and_then(|rows| rows.into_iter().find(|r| r.id == room))
                            .map(|r| r.name)
                            .unwrap_or_else(|| room.to_string());
                        current = current.push(text(name).size(SMALL).style(muted_style));
                    }
                    for entry in project
                        .room_assignments(room)
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|entry| {
                            matches(&entry.poz, &self.assigned_filter)
                                || matches(&entry.item_name, &self.assigned_filter)
                        })
                    {
                        current = current.push(choice(
                            format!(
                                "{} {} · {}",
                                if entry.is_deduction { "−" } else { "+" },
                                entry.poz,
                                entry.item_name
                            ),
                            format!(
                                "{} · {:.2} {}",
                                section_label(entry.section),
                                entry.net,
                                unit_label(entry.unit)
                            ),
                            self.selected_assignment == Some(entry.id),
                            MetrajMsg::SelectRoomAssignment(room, entry.id),
                        ));
                    }
                }
                if let Some(id) = self.selected_assignment {
                    body = body.push(
                        row![
                            action("Seçiliyi güncelle", MetrajMsg::UpdateAssignment, false)
                                .width(Fill),
                            action("Sil", MetrajMsg::RemoveAssignment, false)
                        ]
                        .spacing(4),
                    );
                    // Preserve existing amount deductions in the editor without a separate deduction form.
                    for deduction in project.deductions(id).unwrap_or_default() {
                        current = current.push(
                            row![
                                text(format!(
                                    "{}: −{:.2}",
                                    deduction.description, deduction.quantity
                                ))
                                .size(SMALL)
                                .width(Fill),
                                action("×", MetrajMsg::RemoveDeduction(deduction.id), false)
                                    .width(22)
                            ]
                            .spacing(3),
                        );
                    }
                }
            }
            body = body.push(card(scrollable(current).height(110), false));
        }
        let selected_count = self.pending_items.len() + self.pending_groups.len();
        let apply = action("Uygula", MetrajMsg::ApplyAssignments, true)
            .width(Fill)
            .on_press_maybe(
                (selected_count > 0 && self.selected_room.is_some())
                    .then(|| send(MetrajMsg::ApplyAssignments)),
            );
        let target: Element<'_, Message> = if let Some(room) = self.selected_room {
            mouse_area(self.target_view())
                .on_release(send(MetrajMsg::DropItem(room, None)))
                .into()
        } else {
            self.target_view()
        };
        let mut layout = column![
            target,
            text("⠿ tutamacından mahale, çizime veya hedef mahal alanına sürükleyin.")
                .size(SMALL)
                .style(muted_style),
            scrollable(body).height(Fill),
            row![
                text(format!(
                    "{} kalem · {} grup seçili",
                    self.pending_items.len(),
                    self.pending_groups.len()
                ))
                .size(SMALL)
                .width(Fill),
                action("Temizle", MetrajMsg::ClearCatalogSelection, false)
            ]
            .spacing(4)
            .align_y(iced::Center),
            apply,
            action("İcmal", MetrajMsg::OpenSummary, false).width(Fill),
        ]
        .spacing(4);
        if !self.status.is_empty() {
            layout = layout.push(text(&self.status).size(SMALL));
        }
        crate::ui::dock::frame(
            crate::ui::dock::title_bar(
                PanelId::MetrajAssignments,
                "Metraj atamaları".into(),
                chrome,
            ),
            layout,
            width,
        )
    }
}
