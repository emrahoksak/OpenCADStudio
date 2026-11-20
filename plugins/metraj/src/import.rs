use crate::store::{Group, Item, Project, RebarRow, RebarSettings, Record, Store};
use ocs_plugin_api::dialog::{DialogControl, DialogDef, DialogResponse, DialogValue};
use ocs_plugin_api::host::HostApi;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Result as SqlResult};
use std::path::PathBuf;

pub fn run(host: &mut dyn HostApi, store: &mut Store) -> Result<(), String> {
    let suggested = std::env::var_os("APPDATA")
        .map(|dir| PathBuf::from(dir).join("MHMETRAJ/mhmetraj.db"))
        .filter(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dialog = DialogDef::new("Eski Metraj Projelerini Aktar")
        .with_control(DialogControl::FilePicker {
            id: "database".into(),
            label: "Metraj SQLite dosyası".into(),
            default_path: suggested,
            filter_name: "SQLite".into(),
            extensions: vec!["db".into(), "sqlite".into(), "sqlite3".into()],
            save: false,
        })
        .submit_label("Aktar")
        .cancel_label("Vazgeç");
    let DialogResponse::Submitted(values) = host.show_dialog(dialog) else {
        return Ok(());
    };
    let Some(DialogValue::FilePath(path)) = values.get("database") else {
        return Err("SQLite dosyası seçilmedi.".into());
    };
    let projects =
        read_projects(path).map_err(|error| format!("Metraj veritabanı okunamadı: {error}"))?;
    let mut count = 0;
    for (id, mut project) in projects {
        if !store.imported_projects.insert(id.to_ascii_lowercase()) {
            continue;
        }
        let name = project.name.clone();
        let mut suffix = 2;
        while store
            .projects
            .iter()
            .any(|existing| existing.name.eq_ignore_ascii_case(&project.name))
        {
            project.name = format!("{name} ({suffix})");
            suffix += 1;
        }
        store.projects.push(project);
        count += 1;
    }
    if count != 0 {
        store.save()?;
    }
    host.push_info(&format!("{count} yeni Metraj projesi aktarıldı."));
    Ok(())
}

fn read_projects(path: &str) -> SqlResult<Vec<(String, Project)>> {
    let mut db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let tx = db.transaction()?;
    let mut projects = Vec::new();
    let mut project_query =
        tx.prepare("SELECT Id, Name FROM Projects ORDER BY CreatedAtUtc, Id")?;
    let has_groups = has_column(&tx, "MeasurementGroups", "Id")?;
    let column = |name: &'static str, fallback: &'static str| -> SqlResult<&'static str> {
        Ok(if has_column(&tx, "MeasurementItems", name)? {
            name
        } else {
            fallback
        })
    };
    let sql = format!(
        "SELECT Id, {}, Module, Unit, Name, {}, {} FROM MeasurementItems WHERE ProjectId = ?1 ORDER BY CreatedAtUtc, Id",
        column("GroupId", "NULL")?, column("PozNo", "NULL")?, column("DisplayUnitLabel", "NULL")?
    );
    let mut item_query = tx.prepare(&sql)?;
    for project in project_query.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })? {
        let (id, name) = project?;
        let mut group_ids = Vec::<Option<String>>::new();
        let mut groups = Vec::new();
        if has_groups {
            let mut query = tx.prepare("SELECT Id, Name FROM MeasurementGroups WHERE ProjectId = ?1 ORDER BY CreatedAtUtc, Id")?;
            for group in query.query_map([&id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })? {
                let (group_id, name) = group?;
                group_ids.push(Some(group_id));
                groups.push(Group {
                    name,
                    items: Vec::new(),
                    rebar: Vec::new(),
                });
            }
        }
        let mut items = item_query.query([&id])?;
        while let Some(row) = items.next()? {
            let module: i32 = row.get(2)?;
            if matches!(module, 4 | 5) {
                continue;
            }
            let item_id: String = row.get(0)?;
            let group_id: Option<String> = row.get(1)?;
            let records = read_records(&tx, &item_id)?;
            let rebar = if module == 1 {
                read_rebar(&tx, &item_id)?
            } else {
                Vec::new()
            };
            if module == 1 && records.is_empty() && rebar.is_empty() {
                continue;
            }
            let index = group_id
                .as_ref()
                .and_then(|key| {
                    group_ids
                        .iter()
                        .position(|existing| existing.as_ref() == Some(key))
                })
                .or_else(|| group_ids.iter().position(Option::is_none))
                .unwrap_or_else(|| {
                    group_ids.push(None);
                    let mut name = "Grupsuz eski kayıtlar".to_owned();
                    while groups.iter().any(|group| group.name == name) {
                        name.push_str(" (aktarım)");
                    }
                    groups.push(Group {
                        name,
                        items: Vec::new(),
                        rebar: Vec::new(),
                    });
                    groups.len() - 1
                });
            let group = &mut groups[index];
            group.rebar.extend(rebar);
            if module != 1 || !records.is_empty() {
                let name: String = row.get(4)?;
                group.items.push(Item {
                    name: if module == 1 {
                        format!("{name} (eski manuel)")
                    } else {
                        name
                    },
                    poz: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                    unit: match row.get::<_, i32>(3)? {
                        1 => "kg",
                        2 => "m",
                        3 => "m²",
                        4 => "adet",
                        5 => "ton",
                        6 => "m³",
                        _ => "?",
                    }
                    .into(),
                    display_unit: row
                        .get::<_, Option<String>>(6)?
                        .filter(|label| !label.trim().is_empty()),
                    records,
                });
            }
        }
        projects.push((
            id.clone(),
            Project {
                name,
                groups,
                settings: read_settings(&tx, &id)?,
            },
        ));
    }
    Ok(projects)
}

fn has_column(db: &Connection, table: &str, column: &str) -> SqlResult<bool> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2 COLLATE NOCASE)",
        [table, column],
        |row| row.get(0),
    )
}

fn read_records(db: &Connection, item: &str) -> SqlResult<Vec<Record>> {
    let mut query = db.prepare("SELECT Method, Quantity, Description, SourceText FROM MeasurementRecords WHERE MeasurementItemId = ?1 ORDER BY CreatedAtUtc, Id")?;
    let rows = query.query_map([item], |row| {
        let description: String = row.get(2)?;
        let detail: Option<String> = row.get(3)?;
        Ok(Record {
            method: match row.get::<_, i32>(0)? {
                1 => "Demir",
                2 => "Uzunluk",
                3 => "Alan",
                4 => "Manuel",
                _ => "Eski kayıt",
            }
            .into(),
            quantity: row.get(1)?,
            source: detail.map_or_else(
                || description.clone(),
                |source| format!("{description} | {source}"),
            ),
            handles: Vec::new(),
            original_layers: Vec::new(),
        })
    })?;
    rows.collect()
}

fn read_rebar(db: &Connection, item: &str) -> SqlResult<Vec<RebarRow>> {
    let mut query = db.prepare("SELECT Quantity, DiameterMm, RawLengthCm, TotalWeightKg, OriginalText, DrawingEntityHandle, OriginalLayer FROM RebarMeasurements WHERE MeasurementItemId = ?1 ORDER BY CreatedAtUtc, Id")?;
    let rows = query.query_map([item], |row| {
        let handles: Option<String> = row.get(5)?;
        Ok(RebarRow {
            quantity: row.get(0)?,
            diameter: row.get(1)?,
            length_cm: row.get(2)?,
            weight_kg: row.get(3)?,
            source: row.get(4)?,
            handles: handles
                .unwrap_or_default()
                .split('|')
                .filter_map(|value| u64::from_str_radix(value.trim(), 16).ok())
                .collect(),
            original_layers: row.get::<_, Option<String>>(6)?.into_iter().collect(),
        })
    })?;
    rows.collect()
}

fn read_settings(db: &Connection, project: &str) -> SqlResult<RebarSettings> {
    let mut settings = RebarSettings::default();
    if !has_column(db, "RebarSettings", "Mode")? {
        return Ok(settings);
    }
    let column = |name: &'static str, fallback: &'static str| -> SqlResult<&'static str> {
        Ok(if has_column(db, "RebarSettings", name)? {
            name
        } else {
            fallback
        })
    };
    let sql = format!(
        "SELECT Mode, Symbols, {}, {}, {} FROM RebarSettings WHERE ProjectId = ?1",
        column("MatchColorMode", "0")?,
        column("DrawBoundingBox", "0")?,
        column("MatchColorIndex", "3")?
    );
    if let Some((mode, symbols, mixed, bounds, color)) = db
        .query_row(&sql, [project], |row| {
            Ok((
                row.get::<_, usize>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, u8>(4)?,
            ))
        })
        .optional()?
    {
        settings.mode = mode.saturating_sub(1).min(5);
        settings.symbols = symbols
            .chars()
            .filter(|c| *c != '|' && !c.is_whitespace())
            .collect();
        settings.mixed = mixed;
        settings.bounding_box = bounds;
        if let Some((red, green, blue)) =
            ocs_plugin_api::host::document::types::Color::Index(color).rgb()
        {
            settings.color = [red, green, blue];
        }
    }
    Ok(settings)
}
