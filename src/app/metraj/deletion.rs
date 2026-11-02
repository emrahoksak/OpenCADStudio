use super::{drawing_key, Message, OpenCADStudio};
use codec::Handle;
use iced::Task;

pub(crate) struct PendingRoomDelete {
    tab_id: u64,
    project_id: i64,
    handles: Vec<Handle>,
    rooms: Vec<i64>,
    names: Vec<String>,
    cut: bool,
}

impl OpenCADStudio {
    /// Pause an explicit erase before either the drawing or metraj data changes.
    pub(crate) fn request_metraj_delete(&mut self, handles: &[Handle], cut: bool) -> bool {
        if self.metraj.pending_delete.is_some() {
            return true;
        }
        let Some(project) = self.metraj.project.as_ref() else {
            return false;
        };
        let tab = &self.tabs[self.active_tab];
        let drawing = drawing_key(tab.current_path.as_deref(), tab.id);
        let handles: Vec<_> = handles
            .iter()
            .copied()
            .filter(|h| !tab.scene.is_layer_locked(*h) && !tab.scene.is_reference_only(*h))
            .collect();
        let bindings = match project.bound_room_labels(&drawing) {
            Ok(bindings) => bindings,
            Err(error) => {
                self.command_line.push_error(&format!(
                    "Mahal kayıtları okunamadı; silme iptal edildi: {error}"
                ));
                return true;
            }
        };
        let mut rooms = Vec::new();
        let mut names = Vec::new();
        for (room, boundary, _, name) in bindings {
            if u64::from_str_radix(&boundary, 16)
                .ok()
                .is_some_and(|value| handles.contains(&Handle::new(value)))
            {
                rooms.push(room);
                names.push(name);
            }
        }
        if rooms.is_empty() {
            return false;
        }
        self.metraj.pending_delete = Some(PendingRoomDelete {
            tab_id: tab.id,
            project_id: project.id(),
            handles,
            rooms,
            names,
            cut,
        });
        self.active_modal = Some(crate::app::ModalKind::MetrajDeleteWarning);
        true
    }

    pub(crate) fn metraj_delete_description(&self) -> String {
        let Some(pending) = &self.metraj.pending_delete else {
            return String::new();
        };
        let names = pending
            .names
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} mahalin sınırı silinecek: {}{}\n\nMahal listesindeki kayıtlar, poz atamaları ve minha kayıtları da kalıcı olarak silinecek. Devam edilsin mi?",
            pending.rooms.len(), names, if pending.names.len() > 5 { ", …" } else { "" }
        )
    }

    pub(super) fn confirm_metraj_delete(&mut self) -> Task<Message> {
        if self.active_modal != Some(crate::app::ModalKind::MetrajDeleteWarning) {
            return Task::none();
        }
        let Some(pending) = self.metraj.pending_delete.take() else {
            return Task::none();
        };
        self.close_active_modal();
        let result = self.delete_metraj_rooms(pending);
        if let Err(error) = result {
            self.metraj.status = error.clone();
            self.command_line.push_error(&error);
        }
        Task::none()
    }

    fn delete_metraj_rooms(&mut self, pending: PendingRoomDelete) -> Result<(), String> {
        let i = self.active_tab;
        let tab = &self.tabs[i];
        let project = self
            .metraj
            .project
            .as_ref()
            .ok_or("Silme iptal edildi: proje kapandı.")?;
        if tab.id != pending.tab_id || project.id() != pending.project_id {
            return Err("Silme iptal edildi: etkin çizim veya proje değişti.".into());
        }
        if pending.handles.iter().any(|h| {
            tab.scene.is_layer_locked(*h)
                || tab.scene.is_reference_only(*h)
                || tab.scene.document.get_entity(*h).is_none()
        }) {
            return Err("Silme iptal edildi: seçim değişti veya katman kilitlendi.".into());
        }
        let mut handles = pending.handles.clone();
        for &room in &pending.rooms {
            let (_, _, label, _) = project
                .room_binding(room)
                .map_err(|e| e.to_string())?
                .ok_or("Silme iptal edildi: mahal bağlantısı değişti.")?;
            for hex in [
                label,
                project.room_poz_label(room).map_err(|e| e.to_string())?,
                project.room_hatch_handle(room).map_err(|e| e.to_string())?,
            ]
            .into_iter()
            .flatten()
            {
                if let Ok(value) = u64::from_str_radix(&hex, 16) {
                    let handle = Handle::new(value);
                    if tab.scene.document.get_entity(handle).is_some()
                        && (tab.scene.is_layer_locked(handle)
                            || tab.scene.is_reference_only(handle))
                    {
                        return Err(
                            "Silme iptal edildi: mahal yazısının veya taramasının katmanı kilitli."
                                .into(),
                        );
                    }
                    if tab.scene.document.get_entity(handle).is_some() && !handles.contains(&handle)
                    {
                        handles.push(handle);
                    }
                }
            }
        }
        // Commit SQLite first: a write failure must leave the drawing intact.
        project
            .remove_rooms(&pending.rooms)
            .map_err(|e| format!("Mahaller silinemedi; çizim korunuyor: {e}"))?;
        if pending.cut {
            let base = crate::app::helpers::entities_lower_left_by_bbox(
                &self.tabs[i].scene.document,
                &pending.handles,
            );
            self.copy_entities_to_clipboard(i, &pending.handles, base);
        }
        let delta_safe = self.delta_erase_safe(i, &handles);
        let undo = self.begin_undo(
            i,
            if pending.cut { "CUTCLIP" } else { "ERASE" },
            handles.len(),
            delta_safe,
        );
        if !pending.cut {
            self.oops_cache = handles
                .iter()
                .filter_map(|h| self.tabs[i].scene.document.get_entity_arc(*h))
                .collect();
        }
        self.tabs[i].scene.erase_entities(&handles);
        self.tabs[i].scene.deselect_all();
        self.tabs[i].dirty = true;
        if let Some(undo) = undo {
            self.commit_undo_delta(i, undo);
        }
        self.metraj
            .selected_rooms
            .retain(|room| !pending.rooms.contains(room));
        if self
            .metraj
            .selected_room
            .is_some_and(|room| pending.rooms.contains(&room))
        {
            self.metraj.selected_room = None;
            self.metraj.selected_assignment = None;
        }
        self.metraj
            .expanded_rooms
            .retain(|room| !pending.rooms.contains(room));
        self.metraj
            .expanded_sections
            .retain(|(room, _)| !pending.rooms.contains(room));
        self.metraj.last_sync = None;
        self.metraj.last_selection = None;
        self.metraj.summary_cache.get_mut().take();
        self.metraj.status = format!("{} mahal ve atamaları silindi.", pending.rooms.len());
        self.refresh_properties();
        Ok(())
    }
}
