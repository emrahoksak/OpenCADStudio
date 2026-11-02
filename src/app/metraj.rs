use super::{Message, OpenCADStudio};
use crate::metraj::{NamedRow, Project, QuantityMode, RoomSection, Unit};
use crate::ui::dock::{Chrome, PanelId};
use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{Element, Task};
use std::path::PathBuf;

mod deletion;
mod labels;
mod panels;
mod rebar;
mod rebar_marking;
mod summary_panel;

pub(super) static METRAJ_WINDOW_ID: std::sync::Mutex<Option<iced::window::Id>> =
    std::sync::Mutex::new(None);

#[derive(Debug, Clone)]
pub(crate) enum MetrajMsg {
    Rebar(rebar::RebarMsg),
    Page(MetrajPage),
    NewProject,
    StartMm,
    StartNm,
    ImportClosed,
    AddGeometryDeduction,
    SetRoomTint(RoomSection),
    OpenProject,
    SelectProject(i64),
    ProjectName(String),
    ProjectHeight(String),
    SaveProjectHeight,
    GroupName(String),
    FloorName(String),
    RoomName(String),
    ItemName(String),
    ItemPoz(String),
    PozFilter(String),
    RoomFilter(String),
    GroupFilter(String),
    MemberFilter(String),
    AssignmentFilter(String),
    AssignedFilter(String),
    ToggleTreeForm,
    CatalogGroups(bool),
    ToggleQuantityGroup(i64),
    ToggleCatalogItem(i64),
    ToggleCatalogGroup(i64),
    ClearCatalogSelection,
    ApplyAssignments,
    OpenSummary,
    ExportSummary,
    SummaryFilter(String),
    SelectRoomSection(i64, RoomSection),
    SelectRoomAssignment(i64, i64),
    ItemShortDescription(String),
    QuantityGroupName(String),
    AddQuantityGroup,
    SelectQuantityGroup(i64),
    AddItemToQuantityGroup(i64),
    RemoveItemFromQuantityGroup(i64, i64),
    DragQuantityGroup(i64),
    Length(String),
    Height(String),
    Manual(String),
    Deduction(String),
    DeductionDescription(String),
    AddGroup,
    AddFloor,
    AddRoom,
    AddItem,
    AddItemToSelectedGroup,
    AssignQuantityGroup(i64),
    ToggleItemForm,
    ToggleGroupForm,
    ToggleItemList,
    ToggleGroupList,
    RenameRoom,
    ToggleGroup(i64),
    ToggleFloor(i64),
    ToggleRoom(i64),
    ToggleSection(i64, RoomSection),
    ToggleAssignmentSection(AssignmentPanelSection),
    DragItem(i64),
    CancelDrag,
    DropItem(i64, Option<RoomSection>),
    DragDeduction(bool),
    SelectGroup(i64),
    SelectFloor(i64),
    SelectRoom(i64),
    SelectAssignment(i64),
    Unit(Unit),
    Section(RoomSection),
    Mode(AmountMode),
    AssignItem(i64, bool),
    AddDeduction,
    RemoveDeduction(i64),
    UpdateAssignment,
    RemoveAssignment,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AmountMode {
    Room,
    PerimeterHeight,
    LengthHeight,
    Manual,
}

impl AmountMode {
    fn label(self) -> &'static str {
        match self {
            Self::Room => "Mahal ölçüsü",
            Self::PerimeterHeight => "Kapalı çevre × yükseklik",
            Self::LengthHeight => "Uzunluk × yükseklik",
            Self::Manual => "Elle miktar",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AssignmentPanelSection {
    Settings,
    Catalog,
    Current,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MetrajPage {
    Projects,
    Items,
    Groups,
    Takeoff,
}

pub(crate) struct MetrajUi {
    pub(crate) rebar: rebar::RebarUi,
    pub window: Option<iced::window::Id>,
    page: MetrajPage,
    pub project: Option<Project>,
    project_entries: Vec<NamedRow>,
    pub project_path: Option<PathBuf>,
    pub show_tree: bool,
    pub show_items: bool,
    pub show_quantity_groups: bool,
    pub show_assignments: bool,
    pub show_summary: bool,
    summary_filter: String,
    summary_cache: std::cell::RefCell<Option<(i64, u64, crate::metraj::Summary)>>,
    item_form_open: bool,
    group_form_open: bool,
    item_list_open: bool,
    group_list_open: bool,
    tree_form_open: bool,
    catalog_groups: bool,
    room_filter: String,
    group_filter: String,
    member_filter: String,
    assignment_filter: String,
    assigned_filter: String,
    pending_items: std::collections::BTreeSet<i64>,
    pending_groups: std::collections::BTreeSet<i64>,
    expanded_quantity_groups: std::collections::HashSet<i64>,
    pub selected_group: Option<i64>,
    pub selected_floor: Option<i64>,
    pub selected_room: Option<i64>,
    selected_rooms: std::collections::BTreeSet<i64>,
    pub selected_assignment: Option<i64>,
    expanded_groups: std::collections::HashSet<i64>,
    expanded_floors: std::collections::HashSet<i64>,
    expanded_rooms: std::collections::HashSet<i64>,
    expanded_sections: std::collections::HashSet<(i64, RoomSection)>,
    closed_assignment_sections: std::collections::HashSet<AssignmentPanelSection>,
    pub(crate) dragged_item: Option<i64>,
    pub(crate) dragged_group: Option<i64>,
    selected_quantity_group: Option<i64>,
    quantity_group_name: String,
    drag_deduction: bool,
    project_name: String,
    project_height: String,
    group_name: String,
    floor_name: String,
    room_name: String,
    item_name: String,
    item_poz: String,
    poz_filter: String,
    item_short_description: String,
    unit: Unit,
    section: RoomSection,
    mode: AmountMode,
    length: String,
    height: String,
    manual: String,
    deduction: String,
    deduction_description: String,
    pub status: String,
    pub mm_pending: Option<(u64, i64)>,
    last_selection: Option<(u64, Vec<codec::Handle>)>,
    last_sync: Option<(u64, u64, String)>,
    drawing_paths: std::collections::HashMap<u64, String>,
    pub(super) pending_delete: Option<deletion::PendingRoomDelete>,
}

impl Default for MetrajUi {
    fn default() -> Self {
        Self {
            rebar: rebar::RebarUi::default(),
            window: None,
            page: MetrajPage::Projects,
            project: None,
            project_entries: Vec::new(),
            project_path: None,
            show_tree: false,
            show_items: false,
            show_quantity_groups: false,
            show_assignments: false,
            show_summary: false,
            summary_filter: String::new(),
            summary_cache: std::cell::RefCell::new(None),
            item_form_open: false,
            group_form_open: false,
            item_list_open: true,
            group_list_open: true,
            tree_form_open: false,
            catalog_groups: false,
            room_filter: String::new(),
            group_filter: String::new(),
            member_filter: String::new(),
            assignment_filter: String::new(),
            assigned_filter: String::new(),
            pending_items: std::collections::BTreeSet::new(),
            pending_groups: std::collections::BTreeSet::new(),
            expanded_quantity_groups: std::collections::HashSet::new(),
            selected_group: None,
            selected_floor: None,
            selected_room: None,
            selected_rooms: std::collections::BTreeSet::new(),
            selected_assignment: None,
            expanded_groups: std::collections::HashSet::new(),
            expanded_floors: std::collections::HashSet::new(),
            expanded_rooms: std::collections::HashSet::new(),
            expanded_sections: std::collections::HashSet::new(),
            closed_assignment_sections: std::collections::HashSet::new(),
            dragged_item: None,
            dragged_group: None,
            selected_quantity_group: None,
            quantity_group_name: String::new(),
            drag_deduction: false,
            project_name: String::new(),
            project_height: "2.80".into(),
            group_name: String::new(),
            floor_name: String::new(),
            room_name: String::new(),
            item_name: String::new(),
            item_poz: String::new(),
            poz_filter: String::new(),
            item_short_description: String::new(),
            unit: Unit::SquareMeter,
            section: RoomSection::Floor,
            mode: AmountMode::Room,
            length: String::new(),
            height: String::new(),
            manual: String::new(),
            deduction: String::new(),
            deduction_description: String::new(),
            status: String::new(),
            mm_pending: None,
            last_selection: None,
            last_sync: None,
            drawing_paths: std::collections::HashMap::new(),
            pending_delete: None,
        }
    }
}

impl OpenCADStudio {
    fn room_at_metraj_point(&self, point: [f64; 3]) -> Option<i64> {
        let project = self.metraj.project.as_ref()?;
        let tab = &self.tabs[self.active_tab];
        let drawing = drawing_key(tab.current_path.as_deref(), tab.id);
        project
            .bound_rooms(&drawing)
            .ok()?
            .into_iter()
            .filter_map(|(id, hex)| {
                let handle = codec::Handle::new(u64::from_str_radix(&hex, 16).ok()?);
                let entity = tab.scene.document.get_entity(handle)?;
                let curve = crate::entities::curve::entity_curve(entity)?;
                let uv = curve.plane.project(point)?;
                let inside = kernel::geom2d::contains(
                    &[curve.curve],
                    uv,
                    kernel::geom2d::Tolerance::default(),
                );
                if inside {
                    Some((
                        id,
                        project.room_info(id).ok().flatten()?.measurement.area_m2,
                    ))
                } else {
                    None
                }
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }

    fn assign_metraj_drop(
        &mut self,
        room: i64,
        section: Option<RoomSection>,
    ) -> Result<(), String> {
        let item = self.metraj.dragged_item.take();
        let group = self.metraj.dragged_group.take();
        let project = self.metraj.project.as_ref().ok_or("Önce proje açın.")?;
        let section = section.unwrap_or(self.metraj.section);
        let mode = self.metraj.quantity_mode()?;
        if item.is_none() && group.is_none() {
            return Err("Sürüklenen kalem veya grup yok.".into());
        }
        let mut rooms = self.metraj.target_rooms();
        if !rooms.contains(&room) {
            rooms = vec![room];
        }
        let ids = project
            .assign_rooms(
                &rooms,
                section,
                &item.into_iter().collect::<Vec<_>>(),
                &group.into_iter().collect::<Vec<_>>(),
                mode,
                self.metraj.drag_deduction,
            )
            .map_err(|e| e.to_string())?;
        self.metraj.section = section;
        if rooms.len() == 1 {
            let _ = self.select_metraj_room(room);
        }
        self.metraj.selected_assignment = if rooms.len() == 1 {
            ids.last().map(|(_, id)| *id)
        } else {
            None
        };
        for room in rooms {
            self.metraj.expanded_rooms.insert(room);
            self.metraj.expanded_sections.insert((room, section));
        }
        let tab = &self.tabs[self.active_tab];
        let drawing = drawing_key(tab.current_path.as_deref(), tab.id);
        self.sync_metraj_labels(&drawing);
        Ok(())
    }

    pub(crate) fn drop_metraj_on_canvas(&mut self) -> Task<Message> {
        let point = self.tabs[self.active_tab].last_cursor_world.to_array();
        let result = self
            .room_at_metraj_point(point)
            .ok_or_else(|| "Bu noktada mahal yok.".to_string())
            .and_then(|room| self.assign_metraj_drop(room, None));
        self.metraj.dragged_item = None;
        self.metraj.dragged_group = None;
        self.metraj.status = result.err().unwrap_or_default();
        Task::none()
    }

    fn select_metraj_room(&mut self, room_id: i64) -> Result<(), String> {
        let project = self.metraj.project.as_ref().ok_or("Önce proje açın.")?;
        let location = project
            .room_location(room_id)
            .map_err(|e| e.to_string())?
            .ok_or("Mahal bulunamadı.")?;
        let binding = project.room_binding(room_id).map_err(|e| e.to_string())?;
        self.metraj.selected_group = Some(location.1);
        self.metraj.selected_floor = Some(location.0);
        self.metraj.selected_room = Some(room_id);
        self.metraj.selected_rooms = [room_id].into_iter().collect();
        self.metraj.selected_assignment = None;
        self.metraj.expanded_groups.insert(location.1);
        self.metraj.expanded_floors.insert(location.0);
        self.metraj.expanded_rooms.insert(room_id);
        if let Some((drawing, handle, _, _)) = binding {
            if let Some(index) = self
                .tabs
                .iter()
                .position(|tab| drawing_key(tab.current_path.as_deref(), tab.id) == drawing)
            {
                if index != self.active_tab {
                    let _ = self.update_message(Message::TabSwitch(index));
                }
                if let Ok(value) = u64::from_str_radix(&handle, 16) {
                    let handle = codec::Handle::new(value);
                    if self.tabs[index].scene.document.get_entity(handle).is_some() {
                        self.tabs[index].scene.replace_selection_exact(&[handle]);
                        self.metraj.last_selection = Some((self.tabs[index].id, vec![handle]));
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn sync_metraj_selection(&mut self, clicked_canvas: bool) {
        let Some(project) = self.metraj.project.as_ref() else {
            return;
        };
        let tab = &self.tabs[self.active_tab];
        let tab_id = tab.id;
        let drawing = drawing_key(tab.current_path.as_deref(), tab.id);
        let mut handles = tab.scene.selected_handles_in_order();
        let changed = self.metraj.last_selection.as_ref() != Some(&(tab.id, handles.clone()));
        if !changed && !clicked_canvas {
            return;
        }
        let mut room = handles.iter().find_map(|handle| {
            project
                .room_for_entity(&drawing, &format!("{:X}", handle.value()))
                .ok()
                .flatten()
        });
        if room.is_none() && clicked_canvas && handles.is_empty() && tab.active_cmd.is_none() {
            let point = tab.last_cursor_world.to_array();
            if let Ok(bound) = project.bound_rooms(&drawing) {
                room = bound.into_iter().rev().find_map(|(id, hex)| {
                    let handle = codec::Handle::new(u64::from_str_radix(&hex, 16).ok()?);
                    let entity = tab.scene.document.get_entity(handle)?;
                    let curve = crate::entities::curve::entity_curve(entity)?;
                    let uv = curve.plane.project(point)?;
                    kernel::geom2d::contains(
                        &[curve.curve],
                        uv,
                        kernel::geom2d::Tolerance::default(),
                    )
                    .then_some(id)
                });
            }
            if let Some(id) = room {
                if let Ok(Some((_, hex, _, _))) = project.room_binding(id) {
                    if let Ok(value) = u64::from_str_radix(&hex, 16) {
                        let handle = codec::Handle::new(value);
                        self.tabs[self.active_tab]
                            .scene
                            .replace_selection_exact(&[handle]);
                        handles = vec![handle];
                    }
                }
            }
        }
        let rooms = metraj_rooms_for_handles(project, &drawing, &handles);
        self.metraj.last_selection = Some((tab_id, handles));
        if room.is_some() {
            self.metraj.status = self
                .metraj
                .sync_selected_rooms(rooms)
                .err()
                .unwrap_or_default();
        } else if clicked_canvas || changed {
            self.metraj.selected_room = None;
            self.metraj.selected_rooms.clear();
            self.metraj.selected_assignment = None;
        }
    }
    pub(crate) fn dispatch_metraj(&mut self, command: &str) -> Option<Task<Message>> {
        if let Some(task) = self.dispatch_rebar(command) {
            return Some(task);
        }
        match command {
            "METRAJ" => {
                self.tabs[self.active_tab].last_cmd = Some("METRAJ".into());
                if let Ok(path) = shared_metraj_path() {
                    if path.exists() {
                        self.metraj.project_entries =
                            Project::list_shared(&path).unwrap_or_default();
                    }
                }
                let left = crate::app::config::DockSide::Left;
                let right = crate::app::config::DockSide::Right;
                if !self.dock.is_placed(PanelId::MetrajTree) {
                    self.dock.dock(PanelId::MetrajTree, left, usize::MAX);
                }
                let right_panels = [PanelId::MetrajItems, PanelId::MetrajAssignments];
                self.metraj.show_quantity_groups = false;
                let mut right_group = right_panels.iter().find_map(|id| {
                    self.dock
                        .location(*id)
                        .filter(|(side, _)| *side == right)
                        .map(|(_, group)| group)
                });
                for id in right_panels {
                    if !self.dock.is_placed(id) {
                        if let Some(group) = right_group {
                            self.dock.join_group(id, right, group, usize::MAX);
                        } else {
                            self.dock.dock(id, right, usize::MAX);
                            right_group = self.dock.location(id).map(|(_, group)| group);
                        }
                    }
                }
                for id in [PanelId::MetrajTree, right_panels[0], right_panels[1]] {
                    let _ = self.dock_set_open(id, true);
                }
                self.save_config();
                Some(Task::none())
            }
            "NM" => {
                self.tabs[self.active_tab].last_cmd = Some("NM".into());
                let handles = self.tabs[self.active_tab].scene.selected_handles_in_order();
                let bound_selection = handles.len() == 1
                    && self.metraj.project.as_ref().is_some_and(|project| {
                        let tab = &self.tabs[self.active_tab];
                        project
                            .room_for_boundary(
                                &drawing_key(tab.current_path.as_deref(), tab.id),
                                &format!("{:X}", handles[0].value()),
                            )
                            .ok()
                            .flatten()
                            .is_some()
                    });
                if handles.is_empty() || bound_selection {
                    self.tabs[self.active_tab].scene.deselect_all();
                    let command = crate::modules::draw::select::SelectObjectsCommand::new("NM");
                    self.command_line
                        .push_info(&crate::command::CadCommand::prompt(&command));
                    self.tabs[self.active_tab].active_cmd = Some(Box::new(command));
                    return Some(self.finish_dispatch("NM"));
                } else if handles.len() != 1 {
                    self.command_line
                        .push_error("NM: Tek bir kapalı polyline seçin.");
                } else if let Err(error) =
                    measure_boundary(&self.tabs[self.active_tab].scene.document, handles[0])
                {
                    self.command_line.push_error(&error);
                } else if let Err(error) = self.convert_metraj_boundary(handles[0]) {
                    self.command_line.push_error(&error);
                }
                Some(Task::none())
            }
            "MM" => {
                match self.ensure_metraj_floor() {
                    Ok(floor) => {
                        let command =
                            crate::modules::draw::draw::polyline::PlineCommand::new_metraj();
                        self.command_line
                            .push_info(&crate::command::CadCommand::prompt(&command));
                        self.tabs[self.active_tab].active_cmd = Some(Box::new(command));
                        self.metraj.mm_pending = Some((self.tabs[self.active_tab].id, floor));
                        return Some(self.finish_dispatch("MM"));
                    }
                    Err(error) => self.command_line.push_error(&error),
                }
                Some(Task::none())
            }
            _ => None,
        }
    }

    fn set_metraj_room_tint(&mut self, section: RoomSection) -> Result<(), String> {
        use codec::entities::hatch::{BoundaryEdge, BoundaryPath, PolylineEdge};
        use codec::types::{Color, Transparency, Vector2};
        let room = self.metraj.selected_room.ok_or("Önce mahal seçin.")?;
        let project = self.metraj.project.as_ref().ok_or("Önce proje seçin.")?;
        let (drawing, boundary_hex, _, _) = project
            .room_binding(room)
            .map_err(|e| e.to_string())?
            .ok_or("Mahalin çizim sınırı yok.")?;
        let tab = &mut self.tabs[self.active_tab];
        if drawing_key(tab.current_path.as_deref(), tab.id) != drawing {
            return Err("Mahal çizimini etkinleştirin.".into());
        }
        let color = match section {
            RoomSection::Floor => Color::Index(5),
            RoomSection::Wall => Color::Index(2),
            RoomSection::Ceiling => Color::Index(3),
            RoomSection::Other => return Err("Döşeme, duvar veya tavan seçin.".into()),
        };
        if let Some(hex) = project.room_hatch_handle(room).map_err(|e| e.to_string())? {
            if let Ok(value) = u64::from_str_radix(&hex, 16) {
                if let Some(codec::EntityType::Hatch(existing)) =
                    tab.scene.document.get_entity(codec::Handle::new(value))
                {
                    let mut updated = existing.clone();
                    updated.common.color = color;
                    tab.scene.update_entity(codec::EntityType::Hatch(updated));
                    tab.dirty = true;
                    return Ok(());
                }
            }
        }
        let boundary =
            codec::Handle::new(u64::from_str_radix(&boundary_hex, 16).map_err(|e| e.to_string())?);
        let entity = tab
            .scene
            .document
            .get_entity(boundary)
            .ok_or("Mahal sınırı çizimde yok.")?;
        let curve =
            crate::entities::curve::entity_curve(entity).ok_or("Mahal sınırı ölçülemedi.")?;
        let mut points = curve.curve.tessellate(8.0);
        if points.len() < 3 {
            return Err("Mahal sınırı kapalı değil.".into());
        }
        if points.first() == points.last() {
            points.pop();
        }
        let mut edge = PolylineEdge::new(Vec::new(), true);
        for point in points {
            edge.add_vertex(Vector2::new(point[0], point[1]), 0.0);
        }
        let mut path = BoundaryPath::external();
        path.flags.set_polyline(true);
        path.edges.push(BoundaryEdge::Polyline(edge));
        let mut hatch = codec::entities::Hatch::solid();
        hatch.common.color = color;
        hatch.common.transparency = Transparency::T_40;
        hatch.add_path(path);
        let handle = tab
            .scene
            .add_entity(codec::EntityType::Hatch(Box::new(hatch)));
        project
            .set_room_hatch_handle(room, &format!("{:X}", handle.value()))
            .map_err(|e| e.to_string())?;
        tab.dirty = true;
        Ok(())
    }

    fn add_metraj_geometry_deduction(&mut self) -> Result<(), String> {
        let project = self.metraj.project.as_ref().ok_or("Önce proje seçin.")?;
        let room = self.metraj.selected_room.ok_or("Önce mahal seçin.")?;
        let assignment_id = self.metraj.selected_assignment.ok_or("Önce atama seçin.")?;
        let assignment = project
            .room_assignments(room)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|entry| entry.id == assignment_id)
            .ok_or("Atama bulunamadı.")?;
        let tab = &self.tabs[self.active_tab];
        let handles = tab.scene.selected_handles_in_order();
        if handles.is_empty() {
            return Err("Çizimde minha çizgisi veya kapalı alan seçin.".into());
        }
        let mm_per_unit =
            super::properties::insunits_to_mm(tab.scene.document.header.insertion_units)
                .ok_or("Çizim birimini (INSUNITS) ayarlayın.")?;
        let height = if assignment.unit == Unit::SquareMeter {
            Some(parse_amount(&self.metraj.height)?)
        } else {
            None
        };
        let mut total = 0.0;
        for handle in handles {
            let entity = tab
                .scene
                .document
                .get_entity(handle)
                .ok_or("Seçili çizgi bulunamadı.")?;
            let quantity = match assignment.unit {
                Unit::SquareMeter => match measure_boundary(&tab.scene.document, handle) {
                    Ok(measurement) => measurement.area_m2,
                    Err(_) => {
                        crate::entities::curve::entity_curve(entity)
                            .ok_or("Minha çizgisi ölçülemedi.")?
                            .length()
                            * mm_per_unit
                            / 1000.0
                            * height.unwrap()
                    }
                },
                Unit::Meter => {
                    crate::entities::curve::entity_curve(entity)
                        .ok_or("Minha çizgisi ölçülemedi.")?
                        .length()
                        * mm_per_unit
                        / 1000.0
                }
                Unit::Each => return Err("Adet birimine çizimden minha eklenemez.".into()),
            };
            if !quantity.is_finite() || quantity <= 0.0 {
                return Err("Minha miktarı pozitif olmalı.".into());
            }
            total += quantity;
        }
        project
            .add_deduction(assignment_id, "Çizimden minha", total)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn import_closed_metraj_rooms(&mut self) -> Result<usize, String> {
        let floor = self.ensure_metraj_floor()?;
        let tab = &self.tabs[self.active_tab];
        let drawing_path = drawing_key(tab.current_path.as_deref(), tab.id);
        let boundaries: Vec<_> = tab
            .scene
            .document
            .entities()
            .filter_map(|entity| {
                let handle = entity.common().handle;
                measure_boundary(&tab.scene.document, handle)
                    .ok()
                    .map(|measurement| (handle, measurement))
            })
            .collect();
        let project = self.metraj.project.as_ref().ok_or("Önce proje seçin.")?;
        let mut added = 0;
        for (handle, measurement) in boundaries {
            let hex = format!("{:X}", handle.value());
            if project
                .room_for_boundary(&drawing_path, &hex)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                continue;
            }
            let name = project.next_room_name(floor).map_err(|e| e.to_string())?;
            let room = project
                .add_bound_room(floor, &name, &drawing_path, &hex, measurement)
                .map_err(|e| e.to_string())?;
            self.metraj.selected_room = Some(room);
            added += 1;
        }
        if added > 0 {
            self.sync_metraj_labels(&drawing_path);
            self.metraj.last_sync = None;
        }
        Ok(added)
    }

    fn ensure_metraj_floor(&mut self) -> Result<i64, String> {
        let project = self
            .metraj
            .project
            .as_ref()
            .ok_or("Önce metraj projesi açın.")?;
        if let Some(floor) = self.metraj.selected_floor {
            return Ok(floor);
        }
        let group = if let Some(group) = self.metraj.selected_group {
            group
        } else if let Some(group) = project.groups().map_err(|e| e.to_string())?.first() {
            group.id
        } else {
            project.add_group("Genel").map_err(|e| e.to_string())?
        };
        let floor = if let Some(floor) = project.floors(group).map_err(|e| e.to_string())?.first() {
            floor.id
        } else {
            project
                .add_floor(group, "Zemin Kat")
                .map_err(|e| e.to_string())?
        };
        self.metraj.selected_group = Some(group);
        self.metraj.selected_floor = Some(floor);
        Ok(floor)
    }

    pub(crate) fn finish_mm_boundary(
        &mut self,
        floor: i64,
        handle: codec::Handle,
    ) -> Result<(), String> {
        let tab = &self.tabs[self.active_tab];
        let measurement = measure_boundary(&tab.scene.document, handle)?;
        let drawing_path = drawing_key(tab.current_path.as_deref(), tab.id);
        let project = self
            .metraj
            .project
            .as_ref()
            .ok_or("Önce metraj projesi açın.")?;
        let name = project.next_room_name(floor).map_err(|e| e.to_string())?;
        let room = project
            .add_bound_room(
                floor,
                &name,
                &drawing_path,
                &format!("{:X}", handle.value()),
                measurement,
            )
            .map_err(|e| e.to_string())?;
        self.metraj.selected_room = Some(room);
        self.metraj.expanded_rooms.insert(room);
        self.metraj.expanded_floors.insert(floor);
        if let Ok(Some((_, group))) = project.room_location(room) {
            self.metraj.expanded_groups.insert(group);
        }
        self.metraj.last_sync = None;
        self.sync_metraj_geometry();
        let _ = self.select_metraj_room(room);
        self.command_line.push_info(&format!(
            "{name}: {:.2} m², {:.2} m",
            measurement.area_m2, measurement.perimeter_m
        ));
        Ok(())
    }

    fn convert_metraj_boundary(&mut self, handle: codec::Handle) -> Result<(), String> {
        let project = self
            .metraj
            .project
            .as_ref()
            .ok_or("Önce metraj projesi açın.")?;
        let tab = &self.tabs[self.active_tab];
        let drawing_path = drawing_key(tab.current_path.as_deref(), tab.id);
        let entity_handle = format!("{:X}", handle.value());
        let existing = project
            .room_for_boundary(&drawing_path, &entity_handle)
            .map_err(|error| error.to_string())?;
        if existing.is_some() {
            return Err("Bu sınır zaten bir mahale bağlı.".into());
        }
        let floor = self.ensure_metraj_floor()?;
        self.finish_mm_boundary(floor, handle)
    }

    pub(crate) fn bind_metraj_boundary_to(
        &mut self,
        room: i64,
        handle: codec::Handle,
    ) -> Result<(), String> {
        let project = self
            .metraj
            .project
            .as_ref()
            .ok_or("Önce metraj projesi açın.")?;
        let tab = &self.tabs[self.active_tab];
        let drawing_path = drawing_key(tab.current_path.as_deref(), tab.id);
        let measurement = measure_boundary(&tab.scene.document, handle)?;
        let entity_handle = format!("{:X}", handle.value());
        if project
            .room_for_boundary(&drawing_path, &entity_handle)
            .map_err(|error| error.to_string())?
            .is_some_and(|existing| existing != room)
        {
            return Err("Bu sınır başka bir mahale bağlı.".into());
        }
        project
            .set_room_boundary(room, &drawing_path, &entity_handle, measurement)
            .map_err(|e| e.to_string())?;
        self.metraj.last_sync = None;
        self.command_line.push_info(&format!(
            "Mahal: {:.2} m², {:.2} m",
            measurement.area_m2, measurement.perimeter_m
        ));
        Ok(())
    }

    pub(crate) fn sync_metraj_geometry(&mut self) {
        let Some(project) = &self.metraj.project else {
            return;
        };
        let Some(tab) = self.tabs.get(self.active_tab) else {
            return;
        };
        let drawing_path = drawing_key(tab.current_path.as_deref(), tab.id);
        let key = (tab.id, tab.scene.geometry_epoch, drawing_path.clone());
        if self.metraj.last_sync.as_ref() == Some(&key) {
            return;
        }
        if let Some(old_path) = self.metraj.drawing_paths.get(&tab.id) {
            if old_path != &drawing_path {
                if let Err(error) = project.relink_drawing(old_path, &drawing_path) {
                    self.metraj.status = format!("Çizim bağlantısı taşınamadı: {error}");
                    return;
                }
            }
        }
        self.metraj
            .drawing_paths
            .insert(tab.id, drawing_path.clone());
        let mut missing = 0usize;
        match project.bound_rooms(&drawing_path) {
            Ok(rooms) => {
                for (room_id, hex) in rooms {
                    let measurement = u64::from_str_radix(&hex, 16).ok().and_then(|value| {
                        measure_boundary(&tab.scene.document, codec::Handle::new(value)).ok()
                    });
                    match measurement {
                        Some(m) => {
                            if project
                                .set_room_boundary(room_id, &drawing_path, &hex, m)
                                .is_err()
                            {
                                missing += 1;
                                let _ = project.mark_boundary_missing(room_id);
                            }
                        }
                        None => {
                            missing += 1;
                            let _ = project.mark_boundary_missing(room_id);
                        }
                    }
                }
            }
            Err(error) => {
                self.metraj.status = format!("Mahal bağlantıları okunamadı: {error}");
                return;
            }
        }
        if missing > 0 {
            self.metraj.status = format!("{} mahal sınırı bulunamadı veya geçersiz.", missing);
        } else if self.metraj.status.contains("mahal sınırı bulunamadı") {
            self.metraj.status.clear();
        }
        self.metraj.last_sync = Some(key);
        self.sync_metraj_labels(&drawing_path);
    }

    pub(crate) fn on_metraj(&mut self, msg: MetrajMsg) -> Task<Message> {
        use MetrajMsg::*;
        if let Rebar(message) = msg {
            return self.on_rebar(message);
        }
        if let ConfirmDelete = msg {
            return self.confirm_metraj_delete();
        }
        if let OpenSummary = msg {
            self.dock_open_at_default(PanelId::MetrajSummary);
            return self.dock_set_open(PanelId::MetrajSummary, true);
        }
        if let ExportSummary = msg {
            let result = (|| -> Result<(), String> {
                let summary = self
                    .metraj
                    .project
                    .as_ref()
                    .ok_or("Önce proje seçin.")?
                    .summary()
                    .map_err(|e| e.to_string())?;
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Excel", &["xlsx"])
                    .set_file_name("metraj-icmal.xlsx")
                    .save_file()
                {
                    crate::metraj::export_summary(&path, &summary)?;
                }
                Ok(())
            })();
            self.metraj.status = result.err().unwrap_or_default();
            return Task::none();
        }
        if let ApplyAssignments = msg {
            let result = self.metraj.apply_catalog_selection();
            self.metraj.status = match result {
                Ok(count) => {
                    let tab = &self.tabs[self.active_tab];
                    let drawing = drawing_key(tab.current_path.as_deref(), tab.id);
                    self.sync_metraj_labels(&drawing);
                    format!("{count} atama yapıldı.")
                }
                Err(error) => error,
            };
            return Task::none();
        }
        if let SelectRoomSection(room, section) = msg {
            self.metraj.status = self.select_metraj_room(room).err().unwrap_or_default();
            self.metraj.section = section;
            self.metraj.expanded_sections.insert((room, section));
            return Task::none();
        }
        if let SelectRoomAssignment(room, assignment) = msg {
            if let Err(error) = self.select_metraj_room(room) {
                self.metraj.status = error;
                return Task::none();
            }
            return self.on_metraj(SelectAssignment(assignment));
        }
        if let SetRoomTint(section) = msg {
            self.metraj.status = self.set_metraj_room_tint(section).err().unwrap_or_default();
            return Task::none();
        }
        if let AddGeometryDeduction = msg {
            self.metraj.status = self
                .add_metraj_geometry_deduction()
                .err()
                .unwrap_or_default();
            return Task::none();
        }
        let update_labels = matches!(
            &msg,
            AssignItem(..) | AssignQuantityGroup(..) | RemoveAssignment | UpdateAssignment
        );
        let changed_project = matches!(&msg, NewProject | OpenProject | SelectProject(_));
        if let ImportClosed = msg {
            self.metraj.status = self
                .import_closed_metraj_rooms()
                .map(|count| format!("{count} mahal alındı."))
                .unwrap_or_else(|error| error);
            return Task::none();
        }
        if let StartMm = msg {
            return self.dispatch_metraj("MM").unwrap_or_else(Task::none);
        }
        if let StartNm = msg {
            return self.dispatch_metraj("NM").unwrap_or_else(Task::none);
        }
        if let DropItem(room, section) = msg {
            if self.metraj.dragged_item.is_none() && self.metraj.dragged_group.is_none() {
                return Task::none();
            }
            self.metraj.status = self
                .assign_metraj_drop(room, section)
                .err()
                .unwrap_or_default();
            return Task::none();
        }
        if let SelectRoom(id) = msg {
            self.metraj.status = self.select_metraj_room(id).err().unwrap_or_default();
            return Task::none();
        }
        let state = &mut self.metraj;
        let result: Result<(), String> = (|| {
            match msg {
                StartMm | StartNm | ImportClosed | AddGeometryDeduction | SetRoomTint(_) => {
                    unreachable!()
                }
                ApplyAssignments | SelectRoomSection(..) | SelectRoomAssignment(..) => {
                    unreachable!()
                }
                OpenSummary | ExportSummary | ConfirmDelete | Rebar(_) => unreachable!(),
                SummaryFilter(value) => state.summary_filter = value,
                ToggleTreeForm => state.tree_form_open = !state.tree_form_open,
                CatalogGroups(value) => state.catalog_groups = value,
                RoomFilter(value) => state.room_filter = value,
                GroupFilter(value) => state.group_filter = value,
                MemberFilter(value) => state.member_filter = value,
                AssignmentFilter(value) => state.assignment_filter = value,
                AssignedFilter(value) => state.assigned_filter = value,
                ToggleQuantityGroup(id) => {
                    if !state.expanded_quantity_groups.remove(&id) {
                        state.expanded_quantity_groups.insert(id);
                    }
                }
                ToggleCatalogItem(id) => {
                    if !state.pending_items.remove(&id) {
                        state.pending_items.insert(id);
                    }
                }
                ToggleCatalogGroup(id) => {
                    if !state.pending_groups.remove(&id) {
                        state.pending_groups.insert(id);
                    }
                }
                ClearCatalogSelection => {
                    state.pending_items.clear();
                    state.pending_groups.clear();
                }
                Page(page) => state.page = page,
                NewProject => {
                    if state.project_name.trim().is_empty() {
                        return Err("Proje adı girin.".into());
                    }
                    let height = parse_amount(&state.project_height)?;
                    let path = shared_metraj_path()?;
                    state.project = Some(
                        Project::create_shared(&path, &state.project_name)
                            .map_err(|e| e.to_string())?,
                    );
                    state
                        .project
                        .as_ref()
                        .unwrap()
                        .set_default_height_m(height)
                        .map_err(|e| e.to_string())?;
                    state.height = state.project_height.clone();
                    state.project_entries =
                        Project::list_shared(&path).map_err(|e| e.to_string())?;
                    state.project_path = Some(path);
                    state.last_sync = None;
                    state.drawing_paths.clear();
                    state.selected_group = None;
                    state.selected_floor = None;
                    state.selected_room = None;
                    state.selected_assignment = None;
                    state.mm_pending = None;
                    state.selected_quantity_group = None;
                    state.dragged_group = None;
                    state.last_selection = None;
                    state.expanded_groups.clear();
                    state.expanded_floors.clear();
                    state.expanded_rooms.clear();
                    state.expanded_sections.clear();
                }
                OpenProject => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SQLite", &["sqlite"])
                        .pick_file()
                    {
                        let shared = shared_metraj_path()?;
                        if path == shared {
                            return Err("Bu dosya zaten ortak veritabanı.".into());
                        }
                        let id =
                            Project::import_shared(&shared, &path).map_err(|e| e.to_string())?;
                        state.project =
                            Some(Project::open_shared(&shared, id).map_err(|e| e.to_string())?);
                        state.project_height = state
                            .project
                            .as_ref()
                            .unwrap()
                            .default_height_m()
                            .map_err(|e| e.to_string())?
                            .to_string();
                        state.height = state.project_height.clone();
                        state.project_entries =
                            Project::list_shared(&shared).map_err(|e| e.to_string())?;
                        state.project_path = Some(shared);
                        state.last_sync = None;
                        state.drawing_paths.clear();
                        state.selected_group = None;
                        state.selected_floor = None;
                        state.selected_room = None;
                        state.selected_assignment = None;
                        state.mm_pending = None;
                        state.selected_quantity_group = None;
                        state.dragged_group = None;
                        state.last_selection = None;
                        state.expanded_groups.clear();
                        state.expanded_floors.clear();
                        state.expanded_rooms.clear();
                        state.expanded_sections.clear();
                    }
                }
                SelectProject(id) => {
                    let path = shared_metraj_path()?;
                    state.project =
                        Some(Project::open_shared(&path, id).map_err(|e| e.to_string())?);
                    state.project_height = state
                        .project
                        .as_ref()
                        .unwrap()
                        .default_height_m()
                        .map_err(|e| e.to_string())?
                        .to_string();
                    state.height = state.project_height.clone();
                    state.project_path = Some(path);
                    state.selected_group = None;
                    state.selected_floor = None;
                    state.selected_room = None;
                    state.selected_assignment = None;
                    state.selected_quantity_group = None;
                    state.mm_pending = None;
                    state.last_selection = None;
                    state.last_sync = None;
                    state.drawing_paths.clear();
                    state.expanded_groups.clear();
                    state.expanded_floors.clear();
                    state.expanded_rooms.clear();
                    state.expanded_sections.clear();
                }
                ToggleItemForm => state.item_form_open = !state.item_form_open,
                ToggleGroupForm => state.group_form_open = !state.group_form_open,
                ToggleItemList => state.item_list_open = !state.item_list_open,
                ToggleGroupList => state.group_list_open = !state.group_list_open,
                ProjectName(s) => state.project_name = s,
                ProjectHeight(s) => state.project_height = s,
                SaveProjectHeight => {
                    let project = state.project.as_ref().ok_or("Önce proje seçin.")?;
                    project
                        .set_default_height_m(parse_amount(&state.project_height)?)
                        .map_err(|e| e.to_string())?;
                }
                GroupName(s) => state.group_name = s,
                FloorName(s) => state.floor_name = s,
                RoomName(s) => state.room_name = s,
                ToggleGroup(id) => {
                    if !state.expanded_groups.remove(&id) {
                        state.expanded_groups.insert(id);
                    }
                }
                ToggleFloor(id) => {
                    if !state.expanded_floors.remove(&id) {
                        state.expanded_floors.insert(id);
                    }
                }
                ToggleRoom(id) => {
                    if !state.expanded_rooms.remove(&id) {
                        state.expanded_rooms.insert(id);
                    }
                }
                ToggleSection(id, section) => {
                    if !state.expanded_sections.remove(&(id, section)) {
                        state.expanded_sections.insert((id, section));
                    }
                }
                ToggleAssignmentSection(section) => {
                    if !state.closed_assignment_sections.remove(&section) {
                        state.closed_assignment_sections.insert(section);
                    }
                }
                DragItem(id) => {
                    state.dragged_item = Some(id);
                    state.dragged_group = None;
                }
                DragQuantityGroup(id) => {
                    state.dragged_group = Some(id);
                    state.dragged_item = None;
                }
                CancelDrag => {
                    state.dragged_item = None;
                    state.dragged_group = None;
                }
                QuantityGroupName(name) => state.quantity_group_name = name,
                SelectQuantityGroup(id) => {
                    state.selected_quantity_group = if state.selected_quantity_group == Some(id) {
                        None
                    } else {
                        Some(id)
                    };
                    state.expanded_quantity_groups.insert(id);
                    state.member_filter.clear();
                }
                AddQuantityGroup => {
                    if state.quantity_group_name.trim().is_empty() {
                        return Err("Grup poz adı girin.".into());
                    }
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    state.selected_quantity_group = Some(
                        p.add_quantity_group(&state.quantity_group_name)
                            .map_err(|e| e.to_string())?,
                    );
                    state.quantity_group_name.clear();
                    state.group_form_open = false;
                    if let Some(id) = state.selected_quantity_group {
                        state.expanded_quantity_groups.insert(id);
                    }
                }
                AddItemToQuantityGroup(id) => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    p.add_group_item(
                        state.selected_quantity_group.ok_or("Metraj grubu seçin.")?,
                        id,
                    )
                    .map_err(|e| e.to_string())?;
                }
                RemoveItemFromQuantityGroup(group, item) => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    p.remove_group_item(group, item)
                        .map_err(|e| e.to_string())?;
                }
                DragDeduction(value) => state.drag_deduction = value,
                DropItem(_, _) => unreachable!(),
                ItemName(s) => state.item_name = s,
                ItemPoz(s) => state.item_poz = s,
                PozFilter(s) => state.poz_filter = s,
                ItemShortDescription(s) => state.item_short_description = s,
                Length(s) => state.length = s,
                Height(s) => state.height = s,
                Manual(s) => state.manual = s,
                Deduction(s) => state.deduction = s,
                DeductionDescription(s) => state.deduction_description = s,
                Unit(u) => state.unit = u,
                Section(s) => state.section = s,
                Mode(m) => state.mode = m,
                SelectGroup(id) => {
                    state.selected_group = Some(id);
                    state.expanded_groups.insert(id);
                    state.selected_floor = None;
                    state.selected_room = None;
                    state.selected_assignment = None;
                }
                SelectFloor(id) => {
                    let project = state.project.as_ref().ok_or("Önce proje seçin.")?;
                    let mut parent = None;
                    for group in project.groups().map_err(|error| error.to_string())? {
                        if project
                            .floors(group.id)
                            .map_err(|error| error.to_string())?
                            .iter()
                            .any(|floor| floor.id == id)
                        {
                            parent = Some(group.id);
                            break;
                        }
                    }
                    state.selected_group = Some(parent.ok_or("Kat bulunamadı.")?);
                    state.selected_floor = Some(id);
                    state.expanded_floors.insert(id);
                    state.selected_room = None;
                    state.selected_assignment = None;
                }
                SelectRoom(id) => {
                    let _ = id;
                }
                SelectAssignment(id) => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let room = state.selected_room.ok_or("Mahal seçin.")?;
                    let assignment = p
                        .room_assignments(room)
                        .map_err(|e| e.to_string())?
                        .into_iter()
                        .find(|assignment| assignment.id == id)
                        .ok_or("Metraj ataması bulunamadı.")?;
                    state.selected_assignment = Some(id);
                    state.section = assignment.section;
                    state.drag_deduction = assignment.is_deduction;
                    match assignment.mode {
                        QuantityMode::Room => state.mode = AmountMode::Room,
                        QuantityMode::PerimeterTimesHeight(height_m) => {
                            state.mode = AmountMode::PerimeterHeight;
                            state.length.clear();
                            state.height = height_m.to_string();
                        }
                        QuantityMode::LengthTimesHeight { length_m, height_m } => {
                            state.mode = AmountMode::LengthHeight;
                            state.length = length_m.to_string();
                            state.height = height_m.to_string();
                        }
                        QuantityMode::Manual(quantity) => {
                            state.mode = AmountMode::Manual;
                            state.manual = quantity.to_string();
                        }
                    }
                }
                AddGroup => {
                    if state.group_name.trim().is_empty() {
                        return Err("İş grubu adı girin.".into());
                    }
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    state.selected_group =
                        Some(p.add_group(&state.group_name).map_err(|e| e.to_string())?);
                    state.group_name.clear();
                    state.selected_floor = None;
                    state.selected_room = None;
                    state.selected_assignment = None;
                    if let Some(id) = state.selected_group {
                        state.expanded_groups.insert(id);
                    }
                }
                AddFloor => {
                    if state.floor_name.trim().is_empty() {
                        return Err("Kat adı girin.".into());
                    }
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let group = state.selected_group.ok_or("İş grubu seçin.")?;
                    state.selected_floor = Some(
                        p.add_floor(group, &state.floor_name)
                            .map_err(|e| e.to_string())?,
                    );
                    state.floor_name.clear();
                    state.selected_room = None;
                    state.selected_assignment = None;
                    if let Some(id) = state.selected_floor {
                        state.expanded_floors.insert(id);
                    }
                }
                AddRoom => {
                    if state.room_name.trim().is_empty() {
                        return Err("Mahal adı girin.".into());
                    }
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let floor = state.selected_floor.ok_or("Kat seçin.")?;
                    state.selected_room = Some(
                        p.add_room(floor, &state.room_name)
                            .map_err(|e| e.to_string())?,
                    );
                    state.room_name.clear();
                    if let Some(id) = state.selected_room {
                        state.expanded_rooms.insert(id);
                    }
                }
                RenameRoom => {
                    if state.room_name.trim().is_empty() {
                        return Err("Mahal adı girin.".into());
                    }
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let room = state.selected_room.ok_or("Mahal seçin.")?;
                    p.rename_room(room, &state.room_name)
                        .map_err(|e| e.to_string())?;
                    state.room_name.clear();
                    state.last_sync = None;
                }
                AddItem => {
                    if state.item_name.trim().is_empty() {
                        return Err("Metraj kalemi adı girin.".into());
                    }
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    p.add_item(
                        &state.item_name,
                        state.unit,
                        &state.item_poz,
                        &state.item_short_description,
                    )
                    .map_err(|e| e.to_string())?;
                    state.item_name.clear();
                    state.item_poz.clear();
                    state.item_short_description.clear();
                }
                AddItemToSelectedGroup => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let group = state.selected_quantity_group.ok_or("Grup seçin.")?;
                    if state.item_name.trim().is_empty() {
                        return Err("Metraj kalemi adı girin.".into());
                    }
                    let item = p
                        .add_item(
                            &state.item_name,
                            state.unit,
                            &state.item_poz,
                            &state.item_short_description,
                        )
                        .map_err(|e| e.to_string())?;
                    p.add_group_item(group, item).map_err(|e| e.to_string())?;
                    state.item_name.clear();
                    state.item_poz.clear();
                    state.item_short_description.clear();
                }
                AssignQuantityGroup(group) => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let room = state.selected_room.ok_or("Mahal seçin.")?;
                    let ids = p
                        .assign_group(
                            room,
                            state.section,
                            group,
                            state.quantity_mode()?,
                            state.drag_deduction,
                        )
                        .map_err(|e| e.to_string())?;
                    state.selected_assignment = ids.last().copied();
                }
                AssignItem(item, is_deduction) => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let room = state.selected_room.ok_or("Mahal seçin.")?;
                    let mode = state.quantity_mode()?;
                    state.selected_assignment = Some(
                        p.assign_with_deduction(room, state.section, item, mode, is_deduction)
                            .map_err(|e| e.to_string())?,
                    );
                }
                UpdateAssignment => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let assignment = state.selected_assignment.ok_or("Metraj ataması seçin.")?;
                    p.update_assignment_details(
                        assignment,
                        state.section,
                        state.quantity_mode()?,
                        state.drag_deduction,
                    )
                    .map_err(|e| e.to_string())?;
                }
                AddDeduction => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let assignment = state.selected_assignment.ok_or("Metraj ataması seçin.")?;
                    p.add_deduction(
                        assignment,
                        &state.deduction_description,
                        parse_amount(&state.deduction)?,
                    )
                    .map_err(|e| e.to_string())?;
                    state.deduction.clear();
                    state.deduction_description.clear();
                }
                RemoveDeduction(id) => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    p.remove_deduction(id).map_err(|e| e.to_string())?;
                }
                RemoveAssignment => {
                    let p = state.project.as_ref().ok_or("Önce proje açın.")?;
                    let assignment = state.selected_assignment.ok_or("Metraj ataması seçin.")?;
                    p.remove_assignment(assignment).map_err(|e| e.to_string())?;
                    state.selected_assignment = None;
                }
            }
            Ok(())
        })();
        let success = result.is_ok();
        if success && changed_project {
            let rebar_show = state.rebar.show;
            state.rebar = Default::default();
            state.rebar.show = rebar_show;
            state.selected_rooms.clear();
            state.summary_cache.get_mut().take();
            state.pending_items.clear();
            state.pending_groups.clear();
            state.expanded_quantity_groups.clear();
            state.room_filter.clear();
            state.assigned_filter.clear();
        }
        state.status = result.err().unwrap_or_default();
        if success && changed_project {
            if let Some(project) = &state.project {
                if let Err(error) = state.rebar.load(project) {
                    state.status = error;
                }
            }
        }
        if success && update_labels {
            let tab = &self.tabs[self.active_tab];
            let drawing = drawing_key(tab.current_path.as_deref(), tab.id);
            self.sync_metraj_labels(&drawing);
        }
        Task::none()
    }
}

fn drawing_key(path: Option<&std::path::Path>, tab_id: u64) -> String {
    path.map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("ocs-unsaved-tab:{tab_id}"))
}

fn metraj_rooms_for_handles(
    project: &Project,
    drawing: &str,
    handles: &[codec::Handle],
) -> std::collections::BTreeSet<i64> {
    handles
        .iter()
        .filter_map(|handle| {
            project
                .room_for_entity(drawing, &format!("{:X}", handle.value()))
                .ok()
                .flatten()
        })
        .collect()
}

fn measure_boundary(
    document: &codec::Document,
    handle: codec::Handle,
) -> Result<crate::metraj::Measurement, String> {
    let entity = document
        .get_entity(handle)
        .ok_or("Seçili sınır bulunamadı.")?;
    let closed = match entity {
        codec::EntityType::LwPolyline(polyline) => {
            polyline.is_closed && polyline.vertices.len() >= 3
        }
        codec::EntityType::Polyline2D(polyline) => {
            polyline.is_closed() && polyline.vertices.len() >= 3
        }
        _ => false,
    };
    if !closed {
        return Err("Mahal sınırı kapalı bir 2B polyline veya rectangle olmalı.".into());
    }
    let mm_per_unit = super::properties::insunits_to_mm(document.header.insertion_units)
        .ok_or("Çizim birimini (INSUNITS) ayarlayın.")?;
    let curve = crate::entities::curve::entity_curve(entity).ok_or("Mahal sınırı ölçülemedi.")?;
    let area_m2 = curve.curve.enclosed_area().abs() * (mm_per_unit / 1000.0).powi(2);
    let perimeter_m = curve.length() * mm_per_unit / 1000.0;
    if !area_m2.is_finite() || area_m2 <= 0.0 || !perimeter_m.is_finite() || perimeter_m <= 0.0 {
        return Err("Mahal alanı ve çevresi pozitif olmalı.".into());
    }
    Ok(crate::metraj::Measurement {
        area_m2,
        perimeter_m,
    })
}

fn room_label_geometry(
    entity: &codec::EntityType,
    characters: usize,
) -> Option<(codec::types::Vector3, f64)> {
    let planar = crate::entities::curve::entity_curve(entity)?;
    let points = planar.curve.tessellate(8.0);
    if points.is_empty() {
        return None;
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for point in &points {
        min_x = min_x.min(point[0]);
        max_x = max_x.max(point[0]);
        min_y = min_y.min(point[1]);
        max_y = max_y.max(point[1]);
    }
    let width = max_x - min_x;
    let height = max_y - min_y;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let inside = |point| {
        kernel::geom2d::contains(
            &[planar.curve.clone()],
            point,
            kernel::geom2d::Tolerance::default(),
        )
    };
    let centroid = planar.curve.enclosed_centroid();
    let center = centroid.filter(|point| inside(*point)).or_else(|| {
        let mut best = None;
        for y in 1..10 {
            for x in 1..10 {
                let point = [
                    min_x + width * x as f64 / 10.0,
                    min_y + height * y as f64 / 10.0,
                ];
                if inside(point) {
                    let score = kernel::geom2d::distance_to(&planar.curve, point);
                    if best
                        .as_ref()
                        .is_none_or(|(_, old_score)| score > *old_score)
                    {
                        best = Some((point, score));
                    }
                }
            }
        }
        best.map(|(point, _)| point)
    })?;
    let [x, y, z] = planar.plane.point_at(center);
    Some((
        codec::types::Vector3::new(x, y, z),
        (height.min(width) * 0.025)
            .min(width * 0.65 / (characters.max(1) as f64 * 0.6))
            .max(0.001),
    ))
}

fn parse_amount(input: &str) -> Result<f64, String> {
    input
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or_else(|| "Geçerli, negatif olmayan sayı girin.".into())
}

fn send(msg: MetrajMsg) -> Message {
    Message::Metraj(msg)
}

fn shared_metraj_path() -> Result<PathBuf, String> {
    let dir = crate::config::config_dir().ok_or("Ayar dizini bulunamadı.")?;
    let primary = dir.join("metraj.sqlite");
    if cfg!(debug_assertions) {
        let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("metraj-data")
            .join("metraj.sqlite");
        if fallback.exists() {
            return Ok(fallback);
        }
        if std::fs::create_dir_all(&dir).is_ok()
            && std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&primary)
                .is_ok()
        {
            return Ok(primary);
        }
        std::fs::create_dir_all(fallback.parent().unwrap()).map_err(|e| e.to_string())?;
        if primary.exists() {
            std::fs::copy(&primary, &fallback).map_err(|e| e.to_string())?;
        }
        return Ok(fallback);
    }
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(primary)
}

impl MetrajUi {
    pub(crate) fn window_view(&self) -> Element<'_, Message> {
        use iced::widget::{container, pick_list};
        let chosen = self
            .project_entries
            .iter()
            .find(|entry| self.project.as_ref().map(Project::id) == Some(entry.id))
            .cloned();
        let sidebar = column![
            text("METRAJ").size(25),
            pick_list(chosen, self.project_entries.clone(), |entry: &NamedRow| {
                entry.name.clone()
            })
            .placeholder("Proje seçin")
            .on_select(|entry: NamedRow| send(MetrajMsg::SelectProject(entry.id))),
            button("Projeler").on_press(send(MetrajMsg::Page(MetrajPage::Projects))),
            button("Metraj kalemleri").on_press(send(MetrajMsg::Page(MetrajPage::Items))),
            button("Grup tanımları").on_press(send(MetrajMsg::Page(MetrajPage::Groups))),
            button("Metraj").on_press(send(MetrajMsg::Page(MetrajPage::Takeoff))),
        ]
        .spacing(12)
        .width(220);
        let content: Element<'_, Message> = match self.page {
            MetrajPage::Projects => self.projects_page(),
            MetrajPage::Items => self.items_page(),
            MetrajPage::Groups => self.groups_page(),
            MetrajPage::Takeoff => self.takeoff_page(),
        };
        container(row![sidebar, content].spacing(20).padding(20)).into()
    }

    fn projects_page(&self) -> Element<'_, Message> {
        let mut body = column![
            text("Projeler").size(24),
            row![
                text_input("Yeni proje adı", &self.project_name)
                    .on_input(|s| send(MetrajMsg::ProjectName(s))),
                button("Proje oluştur").on_press(send(MetrajMsg::NewProject))
            ]
            .spacing(8),
            row![
                text_input("Varsayılan yükseklik (m)", &self.project_height)
                    .on_input(|s| send(MetrajMsg::ProjectHeight(s))),
                button("Yüksekliği kaydet").on_press(send(MetrajMsg::SaveProjectHeight))
            ]
            .spacing(8),
            button("Eski SQLite projesini içe aktar").on_press(send(MetrajMsg::OpenProject)),
        ]
        .spacing(12);
        for entry in &self.project_entries {
            body = body.push(
                button(text(entry.name.clone())).on_press(send(MetrajMsg::SelectProject(entry.id))),
            );
        }
        if !self.status.is_empty() {
            body = body.push(text(&self.status));
        }
        scrollable(body).into()
    }

    fn items_page(&self) -> Element<'_, Message> {
        let mut body = column![
            text("Metraj kalemleri").size(24),
            text_input("Poz no", &self.item_poz).on_input(|s| send(MetrajMsg::ItemPoz(s))),
            text_input("Kalem adı", &self.item_name).on_input(|s| send(MetrajMsg::ItemName(s))),
            text_input("Kısa tanım", &self.item_short_description)
                .on_input(|s| send(MetrajMsg::ItemShortDescription(s))),
            row![
                button("m²").on_press(send(MetrajMsg::Unit(Unit::SquareMeter))),
                button("m").on_press(send(MetrajMsg::Unit(Unit::Meter))),
                button("adet").on_press(send(MetrajMsg::Unit(Unit::Each))),
                text(format!("Seçili birim: {}", unit_label(self.unit)))
            ]
            .spacing(8),
            button("Kalem oluştur").on_press(send(MetrajMsg::AddItem)),
        ]
        .spacing(10);
        if let Some(project) = &self.project {
            for item in project.items().unwrap_or_default() {
                body = body.push(text(format!(
                    "{} · {} · {} ({})",
                    item.poz,
                    item.short_description,
                    item.name,
                    unit_label(item.unit)
                )));
            }
        }
        if !self.status.is_empty() {
            body = body.push(text(&self.status));
        }
        scrollable(body).into()
    }

    fn groups_page(&self) -> Element<'_, Message> {
        let mut body = column![
            text("Grup tanımları").size(24),
            row![
                text_input("Grup poz no (DVR-01)", &self.quantity_group_name)
                    .on_input(|s| send(MetrajMsg::QuantityGroupName(s))),
                button("Grup oluştur").on_press(send(MetrajMsg::AddQuantityGroup))
            ]
            .spacing(8),
        ]
        .spacing(10);
        if let Some(project) = &self.project {
            for group in project.quantity_groups().unwrap_or_default() {
                body = body.push(
                    button(text(group.name.clone()))
                        .on_press(send(MetrajMsg::SelectQuantityGroup(group.id))),
                );
                if self.selected_quantity_group == Some(group.id) {
                    for item in project.group_items(group.id).unwrap_or_default() {
                        body = body.push(
                            row![
                                text(format!("{} · {}", item.poz, item.name)),
                                button("Çıkar").on_press(send(
                                    MetrajMsg::RemoveItemFromQuantityGroup(group.id, item.id)
                                ))
                            ]
                            .spacing(8),
                        );
                    }
                }
            }
            if self.selected_quantity_group.is_some() {
                body = body.push(text("Gruba kalem ekle"));
                for item in project.items().unwrap_or_default() {
                    body = body.push(
                        button(text(format!("+ {} · {}", item.poz, item.name)))
                            .on_press(send(MetrajMsg::AddItemToQuantityGroup(item.id))),
                    );
                }
                body = body.push(text("Yeni kalem oluşturup gruba ekle"));
                body = body.push(
                    text_input("Poz no", &self.item_poz).on_input(|s| send(MetrajMsg::ItemPoz(s))),
                );
                body = body.push(
                    text_input("Kalem adı", &self.item_name)
                        .on_input(|s| send(MetrajMsg::ItemName(s))),
                );
                body = body.push(
                    button("Oluştur ve gruba ekle")
                        .on_press(send(MetrajMsg::AddItemToSelectedGroup)),
                );
            }
        }
        if !self.status.is_empty() {
            body = body.push(text(&self.status));
        }
        scrollable(body).into()
    }

    fn takeoff_page(&self) -> Element<'_, Message> {
        let mut rooms = column![
            text("Mahaller").size(20),
            row![
                text_input("İş grubu", &self.group_name)
                    .on_input(|s| send(MetrajMsg::GroupName(s))),
                button("+").on_press(send(MetrajMsg::AddGroup))
            ]
            .spacing(4),
            row![
                text_input("Kat", &self.floor_name).on_input(|s| send(MetrajMsg::FloorName(s))),
                button("+").on_press(send(MetrajMsg::AddFloor))
            ]
            .spacing(4),
            row![
                text_input("Mahal", &self.room_name).on_input(|s| send(MetrajMsg::RoomName(s))),
                button("+").on_press(send(MetrajMsg::AddRoom))
            ]
            .spacing(4),
            button("Polyline ile mahal oluştur (MM)").on_press(send(MetrajMsg::StartMm)),
            button("Seçili kapalı çizgiyi mahal al (NM)").on_press(send(MetrajMsg::StartNm)),
            button("Tüm kapalı polylineleri mahal al").on_press(send(MetrajMsg::ImportClosed))
        ]
        .spacing(8);
        if let Some(project) = &self.project {
            for group in project.groups().unwrap_or_default() {
                rooms = rooms.push(
                    button(text(format!(
                        "{} {}",
                        if self.selected_group == Some(group.id) {
                            "●"
                        } else {
                            "○"
                        },
                        group.name
                    )))
                    .on_press(send(MetrajMsg::SelectGroup(group.id))),
                );
                for floor in project.floors(group.id).unwrap_or_default() {
                    rooms = rooms.push(
                        button(text(format!(
                            "  {} {}",
                            if self.selected_floor == Some(floor.id) {
                                "●"
                            } else {
                                "○"
                            },
                            floor.name
                        )))
                        .on_press(send(MetrajMsg::SelectFloor(floor.id))),
                    );
                    for room in project.rooms(floor.id).unwrap_or_default() {
                        rooms = rooms.push(
                            button(text(format!(
                                "    {} {}",
                                if self.selected_room == Some(room.id) {
                                    "●"
                                } else {
                                    "○"
                                },
                                room.name
                            )))
                            .on_press(send(MetrajMsg::SelectRoom(room.id))),
                        );
                    }
                }
            }
        }
        rooms = rooms.push(
            row![
                button("DŞM · Mavi").on_press(send(MetrajMsg::SetRoomTint(RoomSection::Floor))),
                button("DVR · Sarı").on_press(send(MetrajMsg::SetRoomTint(RoomSection::Wall))),
                button("TVN · Yeşil").on_press(send(MetrajMsg::SetRoomTint(RoomSection::Ceiling)))
            ]
            .spacing(4),
        );
        let mut catalog = column![
            text("Malzemeler ve gruplar").size(20),
            text_input("Poz no ile filtrele", &self.poz_filter)
                .on_input(|s| send(MetrajMsg::PozFilter(s)))
        ]
        .spacing(8);
        if let Some(project) = &self.project {
            for item in project.items().unwrap_or_default() {
                if !item
                    .poz
                    .to_lowercase()
                    .contains(&self.poz_filter.trim().to_lowercase())
                {
                    continue;
                }
                catalog = catalog.push(
                    button(text(format!("{} · {}", item.poz, item.name)))
                        .on_press(send(MetrajMsg::AssignItem(item.id, self.drag_deduction))),
                );
            }
            for group in project.quantity_groups().unwrap_or_default() {
                catalog = catalog.push(
                    button(text(format!("Grup: {}", group.name)))
                        .on_press(send(MetrajMsg::AssignQuantityGroup(group.id))),
                );
            }
        }
        let mut assignment = column![
            text("Atama").size(20),
            text(format!(
                "Seçili mahal: {}",
                self.selected_room
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "Yok".into())
            )),
            button("Döşeme").on_press(send(MetrajMsg::Section(RoomSection::Floor))),
            button("Duvar").on_press(send(MetrajMsg::Section(RoomSection::Wall))),
            button("Tavan").on_press(send(MetrajMsg::Section(RoomSection::Ceiling))),
            button("Mahal alanı / çevresi").on_press(send(MetrajMsg::Mode(AmountMode::Room))),
            button("Uzunluk × yükseklik")
                .on_press(send(MetrajMsg::Mode(AmountMode::PerimeterHeight))),
            text_input("Uzunluk (m)", &self.length).on_input(|s| send(MetrajMsg::Length(s))),
            text(format!(
                "Proje varsayılan yüksekliği: {} m",
                self.project_height
            )),
            text_input("Yükseklik (m)", &self.height).on_input(|s| send(MetrajMsg::Height(s))),
            button("Elle miktar").on_press(send(MetrajMsg::Mode(AmountMode::Manual))),
            text_input("Miktar", &self.manual).on_input(|s| send(MetrajMsg::Manual(s))),
            button(if self.drag_deduction {
                "Minha: açık"
            } else {
                "Minha: kapalı"
            })
            .on_press(send(MetrajMsg::DragDeduction(!self.drag_deduction))),
            text_input("Minha açıklaması", &self.deduction_description)
                .on_input(|s| send(MetrajMsg::DeductionDescription(s))),
            text_input("Minha miktarı", &self.deduction)
                .on_input(|s| send(MetrajMsg::Deduction(s))),
            button("Miktarı minha olarak ekle").on_press(send(MetrajMsg::AddDeduction)),
            button("Seçili çizgi / kapalı alanı minha ekle")
                .on_press(send(MetrajMsg::AddGeometryDeduction)),
            text(&self.status),
        ]
        .spacing(8);
        if let (Some(project), Some(room)) = (&self.project, self.selected_room) {
            assignment = assignment.push(text("Mahaldeki atamalar"));
            for entry in project.room_assignments(room).unwrap_or_default() {
                assignment = assignment.push(
                    button(text(format!(
                        "{}{} · {:.2} {}",
                        if entry.is_deduction { "Minha · " } else { "" },
                        entry.poz,
                        entry.net,
                        unit_label(entry.unit)
                    )))
                    .on_press(send(MetrajMsg::SelectAssignment(entry.id))),
                );
            }
        }
        row![
            scrollable(rooms).width(iced::Length::FillPortion(2)),
            scrollable(catalog).width(iced::Length::FillPortion(2)),
            scrollable(assignment).width(iced::Length::FillPortion(2))
        ]
        .spacing(16)
        .into()
    }

    fn target_rooms(&self) -> Vec<i64> {
        match self.selected_room {
            Some(room) if self.selected_rooms.contains(&room) => {
                self.selected_rooms.iter().copied().collect()
            }
            Some(room) => vec![room],
            None => Vec::new(),
        }
    }
    fn is_target_room(&self, room: i64) -> bool {
        self.selected_room == Some(room)
            || self.selected_room.is_some_and(|primary| {
                self.selected_rooms.contains(&primary) && self.selected_rooms.contains(&room)
            })
    }
    fn sync_selected_rooms(
        &mut self,
        rooms: std::collections::BTreeSet<i64>,
    ) -> Result<(), String> {
        let project = self.project.as_ref().ok_or("Önce proje seçin.")?;
        for room in &rooms {
            if let Some((floor, group)) = project.room_location(*room).map_err(|e| e.to_string())? {
                self.expanded_groups.insert(group);
                self.expanded_floors.insert(floor);
            }
        }
        self.selected_room = rooms.first().copied();
        self.selected_assignment = None;
        if let Some(room) = self.selected_room {
            if let Some((floor, group)) = project.room_location(room).map_err(|e| e.to_string())? {
                self.selected_floor = Some(floor);
                self.selected_group = Some(group);
            }
        }
        self.selected_rooms = rooms;
        Ok(())
    }
    fn apply_catalog_selection(&mut self) -> Result<usize, String> {
        let project = self.project.as_ref().ok_or("Önce proje seçin.")?;
        let rooms = self.target_rooms();
        if rooms.is_empty() {
            return Err("Önce hedef mahal seçin.".into());
        }
        if self.pending_items.is_empty() && self.pending_groups.is_empty() {
            return Err("Uygulamak için kalem veya grup seçin.".into());
        }
        let ids = project
            .assign_rooms(
                &rooms,
                self.section,
                &self.pending_items.iter().copied().collect::<Vec<_>>(),
                &self.pending_groups.iter().copied().collect::<Vec<_>>(),
                self.quantity_mode()?,
                self.drag_deduction,
            )
            .map_err(|error| {
                format!("Atama yapılamadı; kalem birimlerini ve boş grupları kontrol edin. {error}")
            })?;
        self.pending_items.clear();
        self.pending_groups.clear();
        self.selected_assignment = if rooms.len() == 1 {
            ids.last().map(|(_, id)| *id)
        } else {
            None
        };
        for room in rooms {
            self.expanded_rooms.insert(room);
            self.expanded_sections.insert((room, self.section));
        }
        Ok(ids.len())
    }

    fn quantity_mode(&self) -> Result<QuantityMode, String> {
        match self.mode {
            AmountMode::Room => Ok(QuantityMode::Room),
            AmountMode::PerimeterHeight => Ok(QuantityMode::PerimeterTimesHeight(parse_amount(
                &self.height,
            )?)),
            AmountMode::LengthHeight => Ok(QuantityMode::LengthTimesHeight {
                length_m: parse_amount(&self.length)?,
                height_m: parse_amount(&self.height)?,
            }),
            AmountMode::Manual => Ok(QuantityMode::Manual(parse_amount(&self.manual)?)),
        }
    }
}

fn section_label(section: RoomSection) -> &'static str {
    match section {
        RoomSection::Wall => "Duvar",
        RoomSection::Floor => "Döşeme",
        RoomSection::Ceiling => "Tavan",
        RoomSection::Other => "Diğer",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metraj_app() -> OpenCADStudio {
        let mut app = OpenCADStudio::new_for_test();
        app.automation_op(r#"{"op":"new"}"#);
        app.tabs[app.active_tab]
            .scene
            .document
            .header
            .insertion_units = 6;
        let project = Project::open(":memory:", "Metraj commands").unwrap();
        let group = project.add_group("G").unwrap();
        let floor = project.add_floor(group, "F").unwrap();
        app.metraj.project = Some(project);
        app.metraj.selected_group = Some(group);
        app.metraj.selected_floor = Some(floor);
        app
    }

    fn rectangle(app: &mut OpenCADStudio, x: f64) -> codec::Handle {
        use codec::entities::lwpolyline::{LwPolyline, LwVertex};
        let mut polyline = LwPolyline::new();
        polyline.is_closed = true;
        polyline.vertices = [(x, 0.), (x + 4., 0.), (x + 4., 3.), (x, 3.)]
            .into_iter()
            .map(|(x, y)| LwVertex::from_coords(x, y))
            .collect();
        app.tabs[app.active_tab]
            .scene
            .add_entity(codec::EntityType::LwPolyline(polyline))
    }

    #[test]
    fn nm_creates_distinct_automatic_names_and_preserves_previous_room() {
        let mut app = metraj_app();
        let a = rectangle(&mut app, 0.);
        let b = rectangle(&mut app, 10.);
        for handle in [a, b, b] {
            app.tabs[app.active_tab].scene.select_entity(handle, true);
            let _ = app.dispatch_command("NM");
        }
        let project = app.metraj.project.as_ref().unwrap();
        let rooms = project.rooms(app.metraj.selected_floor.unwrap()).unwrap();
        assert_eq!(
            rooms
                .iter()
                .map(|room| room.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Mahal 1", "Mahal 2"]
        );
        assert_eq!(
            project.room_binding(rooms[0].id).unwrap().unwrap().1,
            format!("{:X}", a.value())
        );
        assert_eq!(
            project.room_binding(rooms[1].id).unwrap().unwrap().1,
            format!("{:X}", b.value())
        );
    }

    #[test]
    fn metraj_commands_repeat_with_enter_and_space() {
        let mut app = metraj_app();
        let _ = app.run_command_line("MM 0,0 4,0 4,3 C");
        assert!(app.tabs[app.active_tab].active_cmd.is_none());
        assert!(app.metraj.selected_room.is_some());
        let _ = app.update(Message::CommandSpace);
        assert!(app.metraj.mm_pending.is_some());
        let _ = app.update(Message::CommandEscape);
        app.tabs[app.active_tab].scene.deselect_all();
        for command in ["MM", "NM"] {
            let _ = app.dispatch_command(command);
            assert_eq!(app.tabs[app.active_tab].last_cmd.as_deref(), Some(command));
            for repeat in [
                Message::CommandFinalize,
                Message::CommandSpace,
                Message::CommandSubmit,
            ] {
                let _ = app.update(Message::CommandEscape);
                let _ = app.update(repeat);
                assert!(
                    app.tabs[app.active_tab].active_cmd.is_some(),
                    "{command} did not restart"
                );
                assert_eq!(app.tabs[app.active_tab].last_cmd.as_deref(), Some(command));
                if command == "MM" {
                    assert!(app.metraj.mm_pending.is_some());
                }
            }
            let _ = app.update(Message::CommandEscape);
        }
        let boundary = rectangle(&mut app, 20.);
        app.tabs[app.active_tab].scene.select_entity(boundary, true);
        let _ = app.dispatch_command("NM");
        assert!(app.tabs[app.active_tab].active_cmd.is_none());
        let rooms_before = app
            .metraj
            .project
            .as_ref()
            .unwrap()
            .rooms(app.metraj.selected_floor.unwrap())
            .unwrap()
            .len();
        let _ = app.update(Message::CommandSpace);
        assert!(app.tabs[app.active_tab].active_cmd.is_some());
        assert!(app.tabs[app.active_tab].scene.selected.is_empty());
        assert_eq!(app.tabs[app.active_tab].last_cmd.as_deref(), Some("NM"));
        assert_eq!(
            app.metraj
                .project
                .as_ref()
                .unwrap()
                .rooms(app.metraj.selected_floor.unwrap())
                .unwrap()
                .len(),
            rooms_before
        );
    }

    #[test]
    fn room_delete_warns_cancel_preserves_and_confirm_cleans_records_and_labels() {
        let mut app = metraj_app();
        let boundary = rectangle(&mut app, 0.);
        let other = rectangle(&mut app, 10.);
        app.tabs[app.active_tab].scene.select_entities(&[boundary]);
        let _ = app.dispatch_command("NM");
        let room = app.metraj.selected_room.unwrap();
        let project = app.metraj.project.as_ref().unwrap();
        let item = project
            .add_item("Seramik", Unit::SquareMeter, "01", "")
            .unwrap();
        let assignment = project
            .assign(room, RoomSection::Floor, item, QuantityMode::Room)
            .unwrap();
        project.add_deduction(assignment, "Kapı", 1.).unwrap();
        app.sync_metraj_geometry();
        app.set_metraj_room_tint(RoomSection::Floor).unwrap();
        let project = app.metraj.project.as_ref().unwrap();
        let mut annotation_handles = Vec::new();
        for hex in [
            project.room_binding(room).unwrap().unwrap().2,
            project.room_poz_label(room).unwrap(),
            project.room_hatch_handle(room).unwrap(),
        ]
        .into_iter()
        .flatten()
        {
            annotation_handles.push(codec::Handle::new(u64::from_str_radix(&hex, 16).unwrap()));
        }
        assert_eq!(annotation_handles.len(), 3);
        let poz = app
            .metraj
            .project
            .as_ref()
            .unwrap()
            .room_poz_label(room)
            .unwrap()
            .unwrap();
        let poz = codec::Handle::new(u64::from_str_radix(&poz, 16).unwrap());
        let Some(codec::EntityType::Text(poz)) =
            app.tabs[app.active_tab].scene.document.get_entity(poz)
        else {
            panic!("Poz yazısı yok");
        };
        assert_eq!(poz.height, 3.0);
        assert!(!app.request_metraj_delete(&[other], false));
        assert!(!app.request_metraj_delete(&[annotation_handles[0]], false));
        assert!(app.request_metraj_delete(&[boundary], true));
        assert!(app.clipboard.is_empty());
        let _ = app.update(Message::CloseModal);
        app.tabs[app.active_tab].scene.select_entities(&[boundary]);
        let _ = app.update(Message::DeleteSelected);
        assert_eq!(
            app.active_modal,
            Some(crate::app::ModalKind::MetrajDeleteWarning)
        );
        assert!(app.tabs[app.active_tab]
            .scene
            .document
            .get_entity(boundary)
            .is_some());
        assert_eq!(
            app.metraj
                .project
                .as_ref()
                .unwrap()
                .room_assignments(room)
                .unwrap()
                .len(),
            1
        );
        let _ = app.update(Message::CloseModal);
        assert!(app.metraj.pending_delete.is_none());
        assert!(app
            .metraj
            .project
            .as_ref()
            .unwrap()
            .room_info(room)
            .unwrap()
            .is_some());
        let _ = app.dispatch_command("ERASE");
        assert_eq!(
            app.active_modal,
            Some(crate::app::ModalKind::MetrajDeleteWarning)
        );
        let _ = app.update(Message::Metraj(MetrajMsg::ConfirmDelete));
        assert!(app.tabs[app.active_tab]
            .scene
            .document
            .get_entity(boundary)
            .is_none());
        for handle in annotation_handles {
            assert!(app.tabs[app.active_tab]
                .scene
                .document
                .get_entity(handle)
                .is_none());
        }
        assert!(app.tabs[app.active_tab]
            .scene
            .document
            .get_entity(other)
            .is_some());
        let project = app.metraj.project.as_ref().unwrap();
        assert!(project.room_info(room).unwrap().is_none());
        assert!(project.room_assignments(room).unwrap().is_empty());
        assert!(project.deductions(assignment).unwrap().is_empty());
        assert_eq!(project.items().unwrap().len(), 1);
        assert!(app.metraj.selected_room.is_none());
    }

    #[test]
    fn multi_selection_keeps_rooms_and_uses_each_rooms_measurement() {
        let p = Project::open(":memory:", "Multi").unwrap();
        let g = p.add_group("G").unwrap();
        let f = p.add_floor(g, "F").unwrap();
        let a = p
            .add_bound_room(
                f,
                "A",
                "plan.dwg",
                "A0",
                crate::metraj::Measurement {
                    area_m2: 12.0,
                    perimeter_m: 14.0,
                },
            )
            .unwrap();
        let b = p
            .add_bound_room(
                f,
                "B",
                "plan.dwg",
                "B0",
                crate::metraj::Measurement {
                    area_m2: 20.0,
                    perimeter_m: 18.0,
                },
            )
            .unwrap();
        p.set_label_handle(a, "A1").unwrap();
        p.set_room_poz_label(a, "A2").unwrap();
        p.set_room_hatch_handle(a, "A3").unwrap();
        let item = p.add_item("Seramik", Unit::SquareMeter, "01", "").unwrap();
        let handles = [0xA0, 0xA1, 0xA2, 0xA3, 0xB0, 0xFF].map(codec::Handle::new);
        let rooms = metraj_rooms_for_handles(&p, "plan.dwg", &handles);
        assert_eq!(rooms.len(), 2);
        let mut state = MetrajUi {
            project: Some(p),
            ..Default::default()
        };
        state.sync_selected_rooms(rooms).unwrap();
        state.pending_items.insert(item);
        assert_eq!(state.apply_catalog_selection().unwrap(), 2);
        assert_eq!(state.target_rooms(), vec![a, b]);
        assert!(state.selected_assignment.is_none());
        let p = state.project.as_ref().unwrap();
        assert_eq!(p.room_assignments(a).unwrap()[0].net, 12.0);
        assert_eq!(p.room_assignments(b).unwrap()[0].net, 20.0);
        assert_eq!(p.summary().unwrap().rows[0].net(), 32.0);
    }
    #[test]
    fn multi_room_failure_leaves_no_partial_assignments() {
        let p = Project::open(":memory:", "Multi").unwrap();
        let g = p.add_group("G").unwrap();
        let f = p.add_floor(g, "F").unwrap();
        let room = p.add_room(f, "A").unwrap();
        let item = p.add_item("X", Unit::SquareMeter, "01", "").unwrap();
        assert!(p
            .assign_rooms(
                &[room, 999999],
                RoomSection::Floor,
                &[item],
                &[],
                QuantityMode::Manual(5.0),
                false
            )
            .is_err());
        assert!(p.room_assignments(room).unwrap().is_empty());
        assert_eq!(
            p.assign_rooms(
                &[room, room],
                RoomSection::Floor,
                &[item, item],
                &[],
                QuantityMode::Manual(5.0),
                true
            )
            .unwrap()
            .len(),
            1
        );
    }

    #[test]
    fn staged_catalog_requires_apply_and_clears_after_success() {
        let project = Project::open(":memory:", "Test").unwrap();
        let group = project.add_group("İnşaat").unwrap();
        let floor = project.add_floor(group, "Zemin").unwrap();
        let room = project.add_room(floor, "Banyo").unwrap();
        let item = project
            .add_item("Seramik", Unit::SquareMeter, "01", "")
            .unwrap();
        let mut state = MetrajUi {
            project: Some(project),
            selected_room: Some(room),
            mode: AmountMode::Manual,
            manual: "5".into(),
            ..MetrajUi::default()
        };
        state.pending_items.insert(item);
        assert!(state
            .project
            .as_ref()
            .unwrap()
            .room_assignments(room)
            .unwrap()
            .is_empty());
        assert_eq!(state.apply_catalog_selection().unwrap(), 1);
        assert!(state.pending_items.is_empty());
        assert!(state.apply_catalog_selection().is_err());
        assert_eq!(
            state
                .project
                .as_ref()
                .unwrap()
                .room_assignments(room)
                .unwrap()
                .len(),
            1
        );
        state.pending_items.insert(item);
        state.selected_room = None;
        assert!(state.apply_catalog_selection().is_err());
        assert!(state.pending_items.contains(&item));
    }
}

fn unit_label(unit: Unit) -> &'static str {
    match unit {
        Unit::SquareMeter => "m²",
        Unit::Meter => "m",
        Unit::Each => "adet",
    }
}

inventory::submit!(crate::command::CommandRegistration {
    names: &["METRAJ", "NM", "MM"]
});
