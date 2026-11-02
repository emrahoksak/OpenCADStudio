use super::*;

impl OpenCADStudio {
    pub(super) fn sync_metraj_labels(&mut self, drawing_path: &str) {
        let Some(project) = self.metraj.project.as_ref() else {
            return;
        };
        let Ok(rooms) = project.bound_room_labels(drawing_path) else {
            return;
        };
        let tab = &mut self.tabs[self.active_tab];
        for (room, hex, label_hex, name) in rooms {
            let Some(entity) = u64::from_str_radix(&hex, 16)
                .ok()
                .and_then(|h| tab.scene.document.get_entity(codec::Handle::new(h)))
            else {
                continue;
            };
            let Some((position, height)) = room_label_geometry(entity, name.chars().count()) else {
                continue;
            };
            let mut pozs: Vec<_> = project
                .room_assignments(room)
                .unwrap_or_default()
                .into_iter()
                .map(|a| a.poz)
                .filter(|p| !p.trim().is_empty())
                .collect();
            pozs.sort();
            pozs.dedup();
            let pozs = pozs.join(", ");
            let poz_height = 3.0;
            let mut upsert =
                |old: Option<String>, value: &str, position: codec::types::Vector3, height: f64| {
                    let old = old
                        .and_then(|s| u64::from_str_radix(&s, 16).ok())
                        .map(codec::Handle::new);
                    if let Some(handle) = old {
                        if let Some(codec::EntityType::Text(existing)) =
                            tab.scene.document.get_entity(handle)
                        {
                            if existing.value == value
                                && (existing.height - height).abs() < 1e-7
                                && (existing.insertion_point.x - position.x).abs()
                                    + (existing.insertion_point.y - position.y).abs()
                                    + (existing.insertion_point.z - position.z).abs()
                                    < 1e-7
                            {
                                return handle;
                            }
                            let mut text = existing.clone();
                            text.value = value.into();
                            text.height = height;
                            text.insertion_point = position;
                            text.alignment_point = Some(position);
                            tab.scene.update_entity(codec::EntityType::Text(text));
                            tab.dirty = true;
                            return handle;
                        }
                    }
                    let mut text =
                        codec::entities::Text::with_value(value, position).with_height(height);
                    text.alignment_point = Some(position);
                    text.horizontal_alignment =
                        codec::entities::text::TextHorizontalAlignment::Center;
                    text.vertical_alignment = codec::entities::text::TextVerticalAlignment::Middle;
                    let handle = tab.scene.add_entity(codec::EntityType::Text(text));
                    tab.dirty = true;
                    handle
                };
            let name_handle = upsert(label_hex, &name, position, height);
            let _ = project.set_label_handle(room, &format!("{:X}", name_handle.value()));
            let old_poz = project.room_poz_label(room).ok().flatten();
            if !pozs.is_empty() || old_poz.is_some() {
                let lower =
                    codec::types::Vector3::new(position.x, position.y - height * 1.6, position.z);
                let handle = upsert(old_poz, &pozs, lower, poz_height);
                let _ = project.set_room_poz_label(room, &format!("{:X}", handle.value()));
            }
        }
    }
}
