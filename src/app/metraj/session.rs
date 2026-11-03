use super::{drawing_key, shared_metraj_path, MetrajUi, OpenCADStudio, PanelId, Project};
use std::path::Path;

impl MetrajUi {
    pub(super) fn restore_from(&mut self, path: &Path) -> Result<(), String> {
        self.project_entries = Project::list_shared(path).map_err(|error| error.to_string())?;
        let Some(project) = Project::restore_shared(path).map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        self.project_height = project
            .default_height_m()
            .map_err(|error| error.to_string())?
            .to_string();
        self.height = self.project_height.clone();
        self.room_fill_enabled = project.room_fill_enabled().map_err(|error| error.to_string())?;
        (self.room_fill_color, self.room_fill_transparency) = project.room_fill_style().map_err(|error| error.to_string())?;
        self.selected_group = project
            .groups()
            .map_err(|error| error.to_string())?
            .first()
            .map(|row| row.id);
        self.selected_floor = match self.selected_group {
            Some(group) => project
                .floors(group)
                .map_err(|error| error.to_string())?
                .first()
                .map(|row| row.id),
            None => None,
        };
        self.expanded_groups.extend(self.selected_group);
        self.expanded_floors.extend(self.selected_floor);
        self.project_path = Some(path.to_owned());
        self.last_sync = None;
        self.project = Some(project);
        self.rebar.load(self.project.as_ref().unwrap())
    }
}

impl OpenCADStudio {
    pub(crate) fn restore_metraj_session(&mut self) {
        let result = shared_metraj_path().and_then(|path| self.metraj.restore_from(&path));
        if let Err(error) = result {
            self.metraj.status = format!("Metraj projesi açılamadı: {error}");
            self.command_line.push_error(&self.metraj.status);
        } else {
            self.metraj.status.clear();
        }
        if self.metraj.project.is_some() {
            self.metraj.show_tree = self.dock.is_placed(PanelId::MetrajTree);
            self.metraj.show_items = self.dock.is_placed(PanelId::MetrajItems);
            self.metraj.show_assignments = self.dock.is_placed(PanelId::MetrajAssignments);
            self.metraj.show_summary = self.dock.is_placed(PanelId::MetrajSummary);
            self.metraj.rebar.show = self.dock.is_placed(PanelId::MetrajRebar);
        }
    }

    // Save-and-close removes the tab before the normal geometry sync can run.
    pub(crate) fn relink_metraj_after_save(
        &mut self,
        tab_index: usize,
        previous_path: Option<&Path>,
        path: &Path,
    ) -> Result<(), String> {
        let Some(project) = &self.metraj.project else {
            return Ok(());
        };
        let tab = &self.tabs[tab_index];
        let previous = self
            .metraj
            .drawing_paths
            .entry(tab.id)
            .or_insert_with(|| drawing_key(previous_path, tab.id));
        let current = drawing_key(Some(path), tab.id);
        if *previous != current {
            project
                .relink_drawing(previous, &current)
                .map_err(|error| format!("Metraj çizim bağlantısı kaydedilemedi: {error}"))?;
            *previous = current;
            self.metraj.last_sync = None;
            if self
                .metraj
                .status
                .starts_with("Metraj çizim bağlantısı kaydedilemedi:")
            {
                self.metraj.status.clear();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Message, SaveContinuation, SaveOutcome, SavePurpose};
    use crate::metraj::{Measurement, QuantityMode, RoomSection, Unit};

    fn path(extension: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "ocs-metraj-session-{}-{nonce}.{extension}",
            std::process::id()
        ))
    }

    fn fixture(database: &Path) -> (OpenCADStudio, i64, codec::Handle) {
        let mut app = OpenCADStudio::new_for_test();
        app.automation_op(r#"{"op":"new"}"#);
        let i = app.active_tab;
        app.tabs[i].scene.document.header.insertion_units = 6;
        let mut boundary = codec::entities::lwpolyline::LwPolyline::new();
        boundary.is_closed = true;
        boundary.vertices = [(0., 0.), (4., 0.), (4., 3.), (0., 3.)]
            .into_iter()
            .map(|(x, y)| codec::entities::lwpolyline::LwVertex::from_coords(x, y))
            .collect();
        let handle = app.tabs[i]
            .scene
            .add_entity(codec::EntityType::LwPolyline(boundary));
        let project = Project::create_shared(database, "Kalıcı proje").unwrap();
        project.set_default_height_m(3.2).unwrap();
        let group = project.add_group("İnşaat").unwrap();
        let floor = project.add_floor(group, "Zemin").unwrap();
        let drawing = drawing_key(None, app.tabs[i].id);
        let room = project
            .add_bound_room(
                floor,
                "Banyo",
                &drawing,
                &format!("{:X}", handle.value()),
                Measurement {
                    area_m2: 12.,
                    perimeter_m: 14.,
                },
            )
            .unwrap();
        let item = project
            .add_item("Seramik", Unit::SquareMeter, "01", "")
            .unwrap();
        project
            .assign(room, RoomSection::Floor, item, QuantityMode::Room)
            .unwrap();
        project
            .add_rebar(&[crate::metraj::rebar::RebarEntry::manual(2, 16, 250., false).unwrap()])
            .unwrap();
        app.metraj.project = Some(project);
        app.metraj.project_path = Some(database.to_owned());
        app.sync_metraj_geometry();
        (app, room, handle)
    }

    #[test]
    fn metraj_session_restores_selected_project_and_existing_records() {
        let database = path("sqlite");
        let (app, room, _) = fixture(&database);
        let id = app.metraj.project.as_ref().unwrap().id();
        app.metraj.project.as_ref().unwrap().set_room_fill_style([20, 80, 160], 55).unwrap();
        let other = Project::create_shared(&database, "Diğer proje").unwrap();
        drop(other);
        drop(Project::open_shared(&database, id).unwrap());
        drop(app);

        let mut restored = MetrajUi::default();
        restored.restore_from(&database).unwrap();
        let project = restored.project.as_ref().unwrap();
        assert_eq!(project.id(), id);
        assert_eq!(restored.project_entries.len(), 2);
        assert_eq!(project.groups().unwrap().len(), 1);
        assert_eq!(project.items().unwrap().len(), 1);
        assert_eq!(project.room_assignments(room).unwrap().len(), 1);
        assert_eq!(project.rebar_rows().unwrap().len(), 1);
        assert_eq!(restored.height, "3.2");
        assert_eq!(restored.room_fill_color, [20, 80, 160]);
        assert_eq!(restored.room_fill_transparency, 55);
        assert!(restored.selected_floor.is_some());
        drop(restored);
        std::fs::remove_file(database).unwrap();
    }

    #[test]
    fn metraj_session_legacy_database_upgrade_keeps_all_records() {
        let database = path("sqlite");
        let (app, room, _) = fixture(&database);
        let id = app.metraj.project.as_ref().unwrap().id();
        drop(app);
        {
            let db = rusqlite::Connection::open(&database).unwrap();
            db.execute_batch("DROP TABLE metraj_session").unwrap();
        }
        for _ in 0..2 {
            let project = Project::restore_shared(&database).unwrap().unwrap();
            assert_eq!(project.id(), id);
            assert_eq!(project.groups().unwrap().len(), 1);
            assert_eq!(project.items().unwrap().len(), 1);
            assert_eq!(project.room_assignments(room).unwrap().len(), 1);
            assert_eq!(project.rebar_rows().unwrap().len(), 1);
        }
        // Older readers still see the same tables and rows.
        let db = rusqlite::Connection::open(&database).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM room", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(db);
        std::fs::remove_file(database).unwrap();
    }

    #[test]
    fn metraj_session_save_and_close_reopens_valid_room_and_quantities() {
        let database = path("sqlite");
        let drawing = path("dxf");
        let (mut app, room, handle) = fixture(&database);
        let i = app.active_tab;
        let tab_id = app.tabs[i].id;
        crate::io::save(&app.tabs[i].scene.document, &drawing).unwrap();
        app.active_save_jobs.insert(tab_id, 1);
        let outcome = SaveOutcome {
            job_id: 1,
            tab_id,
            epoch: app.tabs[i].scene.geometry_epoch,
            revision: app.tabs[i].edit_revision,
            camera_generation: app.tabs[i].scene.camera_generation,
            path: drawing.clone(),
            version: app.tabs[i].scene.document.version,
            previous_autosave: None,
            set_current_path: true,
            purpose: SavePurpose::SaveAs,
            continuation: SaveContinuation::CloseTab,
            refreshed_preview: None,
            result: Ok(()),
        };
        let old_key = drawing_key(None, tab_id);
        let blocker = rusqlite::Connection::open(&database).unwrap();
        blocker
            .execute_batch(
                "CREATE TRIGGER reject_relink BEFORE UPDATE OF drawing_path ON room
            BEGIN SELECT RAISE(FAIL, 'test write failure'); END",
            )
            .unwrap();
        let _ = app.update(Message::SaveFinished(outcome.clone()));
        assert!(app.tabs.iter().any(|tab| tab.id == tab_id));
        assert_eq!(
            app.metraj
                .project
                .as_ref()
                .unwrap()
                .room_for_boundary(&old_key, &format!("{:X}", handle.value()))
                .unwrap(),
            Some(room)
        );
        blocker.execute_batch("DROP TRIGGER reject_relink").unwrap();
        drop(blocker);
        app.active_save_jobs.insert(tab_id, 1);
        let _ = app.update(Message::SaveFinished(outcome));
        assert!(!app.tabs.iter().any(|tab| tab.id == tab_id));
        drop(app);

        let mut reopened = OpenCADStudio::new_for_test();
        reopened.metraj.restore_from(&database).unwrap();
        let reply =
            reopened.automation_op(&serde_json::json!({"op":"open", "path":drawing}).to_string());
        assert_eq!(reply["ok"], true, "{reply}");
        reopened.sync_metraj_geometry();
        let project = reopened.metraj.project.as_ref().unwrap();
        assert_eq!(
            project
                .room_for_boundary(&drawing.to_string_lossy(), &format!("{:X}", handle.value()))
                .unwrap(),
            Some(room)
        );
        assert!(project.room_info(room).unwrap().unwrap().boundary_valid);
        assert_eq!(project.room_assignments(room).unwrap()[0].net, 12.);
        assert_eq!(project.rebar_rows().unwrap().len(), 1);
        drop(reopened);
        std::fs::remove_file(database).unwrap();
        std::fs::remove_file(drawing).unwrap();
    }

    #[test]
    fn metraj_session_synchronous_save_relinks_before_returning() {
        let database = path("sqlite");
        let drawing = path("dwg");
        let (mut app, room, handle) = fixture(&database);
        let i = app.active_tab;
        app.save_tab_synchronously_protected(i, drawing.clone(), true)
            .unwrap();
        let project = app.metraj.project.as_ref().unwrap();
        assert_eq!(
            project
                .room_for_boundary(&drawing.to_string_lossy(), &format!("{:X}", handle.value()))
                .unwrap(),
            Some(room)
        );
        drop(app);
        std::fs::remove_file(database).unwrap();
        std::fs::remove_file(drawing).unwrap();
    }

    #[test]
    fn metraj_session_does_not_rebind_unsaved_rooms_from_an_earlier_run() {
        let database = path("sqlite");
        let (mut app, room, handle) = fixture(&database);
        let id = app.metraj.project.as_ref().unwrap().id();
        let old_key = format!("ocs-unsaved-tab:{}", app.tabs[app.active_tab].id);
        app.metraj
            .project
            .as_ref()
            .unwrap()
            .set_room_boundary(
                room,
                &old_key,
                &format!("{:X}", handle.value()),
                Measurement {
                    area_m2: 12.,
                    perimeter_m: 14.,
                },
            )
            .unwrap();
        app.metraj.project = Some(Project::open_shared(&database, id).unwrap());
        app.metraj.last_sync = None;
        app.sync_metraj_geometry();
        assert!(
            !app.metraj
                .project
                .as_ref()
                .unwrap()
                .room_info(room)
                .unwrap()
                .unwrap()
                .boundary_valid
        );
        assert_ne!(drawing_key(None, app.tabs[app.active_tab].id), old_key);
        assert_eq!(
            shared_metraj_path().unwrap(),
            crate::config::config_dir().unwrap().join("metraj.sqlite")
        );
        drop(app);
        std::fs::remove_file(database).unwrap();
    }
}
