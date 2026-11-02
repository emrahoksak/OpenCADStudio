use super::{rebar::*, Project};
use rusqlite::{params, OptionalExtension};

impl Project {
    pub fn rebar_settings(&self) -> rusqlite::Result<RebarSettings> {
        self.db
            .query_row(
                "SELECT mode,symbols,multiplier,tolerance,marking_mode,marking_color,recolor_text,draw_boxes FROM rebar_settings WHERE project_id=?1",
                [self.id],
                |r| {
                    let mode: i64 = r.get(0)?;
                    Ok(RebarSettings {
                        mode: *RebarMode::ALL
                            .get(mode as usize)
                            .ok_or(rusqlite::Error::InvalidQuery)?,
                        symbols: r.get(1)?,
                        multiplier: r.get(2)?,
                        tolerance: r.get(3)?,
                        marking: *RebarMarking::ALL.get(r.get::<_, i64>(4)? as usize).ok_or(rusqlite::Error::InvalidQuery)?,
                        color: r.get(5)?,
                        recolor_text: r.get(6)?,
                        draw_boxes: r.get(7)?,
                    })
                },
            )
            .optional()
            .map(|value| value.unwrap_or_default())
    }
    pub fn save_rebar_settings(&self, settings: &RebarSettings) -> rusqlite::Result<()> {
        settings
            .validate()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mode = RebarMode::ALL
            .iter()
            .position(|m| *m == settings.mode)
            .unwrap() as i32;
        let marking = RebarMarking::ALL
            .iter()
            .position(|m| *m == settings.marking)
            .unwrap() as i32;
        self.db.execute("INSERT INTO rebar_settings(project_id,mode,symbols,multiplier,tolerance,marking_mode,marking_color,recolor_text,draw_boxes) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(project_id) DO UPDATE SET mode=excluded.mode,symbols=excluded.symbols,multiplier=excluded.multiplier,tolerance=excluded.tolerance,marking_mode=excluded.marking_mode,marking_color=excluded.marking_color,recolor_text=excluded.recolor_text,draw_boxes=excluded.draw_boxes", params![self.id, mode, settings.symbols, settings.multiplier, settings.tolerance, marking, settings.color, settings.recolor_text, settings.draw_boxes])?;
        Ok(())
    }
    pub fn add_rebar(&self, rows: &[RebarEntry]) -> rusqlite::Result<()> {
        if rows.is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let tx = self.db.unchecked_transaction()?;
        for row in rows {
            row.validate().map_err(|_| rusqlite::Error::InvalidQuery)?;
            self.db.execute("INSERT INTO rebar_measurement(project_id,quantity,diameter_mm,length_cm,deduction,source,drawing,handles,layer) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![self.id, row.quantity, row.diameter_mm, row.length_cm, row.deduction, row.source, row.drawing, row.handles, row.layer])?;
        }
        tx.commit()
    }
    pub fn rebar_rows(&self) -> rusqlite::Result<Vec<RebarEntry>> {
        let mut statement = self.db.prepare("SELECT id,quantity,diameter_mm,length_cm,deduction,source,drawing,handles,layer FROM rebar_measurement WHERE project_id=?1 ORDER BY id")?;
        let rows = statement
            .query_map([self.id], |r| {
                Ok(RebarEntry {
                    id: r.get(0)?,
                    quantity: r.get(1)?,
                    diameter_mm: r.get(2)?,
                    length_cm: r.get(3)?,
                    deduction: r.get(4)?,
                    source: r.get(5)?,
                    drawing: r.get(6)?,
                    handles: r.get(7)?,
                    layer: r.get(8)?,
                })
            })?
            .collect();
        rows
    }
    pub fn remove_rebar(&self, id: i64) -> rusqlite::Result<()> {
        if self.db.execute(
            "DELETE FROM rebar_measurement WHERE id=?1 AND project_id=?2",
            params![id, self.id],
        )? != 1
        {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }
    pub fn update_rebar(&self, id: i64, entry: &RebarEntry) -> rusqlite::Result<()> {
        entry
            .validate()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        if self.db.execute("UPDATE rebar_measurement SET quantity=?1,diameter_mm=?2,length_cm=?3,deduction=?4 WHERE id=?5 AND project_id=?6", params![entry.quantity,entry.diameter_mm,entry.length_cm,entry.deduction,id,self.id])? != 1 { return Err(rusqlite::Error::QueryReturnedNoRows); }
        Ok(())
    }
}

pub fn export_rebar(path: &std::path::Path, rows: &[RebarEntry]) -> Result<(), String> {
    use super::summary::{write_sheet, xml};
    let mut sheet = String::from("<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" state=\"frozen\"/></sheetView></sheetViews><cols><col min=\"1\" max=\"9\" width=\"18\" customWidth=\"1\"/><col min=\"10\" max=\"12\" width=\"45\" customWidth=\"1\"/></cols><sheetData>");
    let mut line = 0usize;
    let mut append = |values: Vec<Result<f64, String>>| {
        line += 1;
        sheet.push_str(&format!("<row r=\"{line}\">"));
        for (column, value) in values.into_iter().enumerate() {
            let cell = format!("{}{line}", (b'A' + column as u8) as char);
            match value {
                Ok(value) => sheet.push_str(&format!("<c r=\"{cell}\"><v>{value}</v></c>")),
                Err(value) => sheet.push_str(&format!(
                    "<c r=\"{cell}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                    xml(&value)
                )),
            }
        }
        sheet.push_str("</row>");
    };
    append(
        [
            "Adet",
            "Çap (mm)",
            "Birim boy (cm)",
            "Toplam boy (m)",
            "Birim ağırlık (kg/m)",
            "Ekle (kg)",
            "Minha (kg)",
            "Net (kg)",
            "Tip",
            "Kaynak metin",
            "Çizim",
            "Kaynak handle",
        ]
        .iter()
        .map(|s| Err((*s).into()))
        .collect(),
    );
    for r in rows {
        append(vec![
            Ok(r.quantity as f64),
            Ok(r.diameter_mm as f64),
            Ok(r.length_cm),
            Ok(r.total_length_m()),
            Ok((r.diameter_mm as f64).powi(2) / 162.),
            Ok(if r.deduction { 0. } else { r.weight_kg() }),
            Ok(if r.deduction { -r.weight_kg() } else { 0. }),
            Ok(r.weight_kg()),
            Err(if r.deduction { "Minha" } else { "Ekle" }.into()),
            Err(r.source.clone()),
            Err(r.drawing.clone()),
            Err(r.handles.clone()),
        ]);
    }
    append(vec![]);
    append(
        [
            "Çap (mm)",
            "Net boy (m)",
            "Ekle (kg)",
            "Minha (kg)",
            "Net (kg)",
            "Net (ton)",
        ]
        .iter()
        .map(|s| Err((*s).into()))
        .collect(),
    );
    let summary = summarize_rebar(rows);
    for r in &summary {
        append(vec![
            Ok(r.diameter_mm as f64),
            Ok(r.length_m),
            Ok(r.added_kg),
            Ok(r.deducted_kg),
            Ok(r.net_kg()),
            Ok(r.net_kg() / 1000.),
        ]);
    }
    append(vec![
        Err("TOPLAM".into()),
        Ok(summary.iter().map(|r| r.length_m).sum()),
        Ok(summary.iter().map(|r| r.added_kg).sum()),
        Ok(summary.iter().map(|r| r.deducted_kg).sum()),
        Ok(summary.iter().map(|r| r.net_kg()).sum()),
        Ok(summary.iter().map(|r| r.net_kg() / 1000.).sum()),
    ]);
    sheet.push_str("</sheetData></worksheet>");
    write_sheet(path, sheet)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "ocs-rebar-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn rebar_persists_without_rooms_is_project_scoped_and_atomic() {
        let path = path("shared.sqlite");
        let p = Project::create_shared(&path, "A").unwrap();
        let id = p.id();
        let q = Project::create_shared(&path, "B").unwrap();
        let mut plus = RebarEntry::manual(12, 16, 250., false).unwrap();
        plus.drawing = "old-plan.dwg".into();
        let minus = RebarEntry::manual(2, 16, 250., true).unwrap();
        p.add_rebar(&[plus.clone(), minus]).unwrap();
        let row_id = p.rebar_rows().unwrap()[0].id;
        assert!(p.groups().unwrap().is_empty());
        assert!(q.rebar_rows().unwrap().is_empty());
        assert!(q.remove_rebar(row_id).is_err());
        assert!(q.update_rebar(row_id, &plus).is_err());
        let mut invalid = plus.clone();
        invalid.length_cm = -5.;
        assert!(p.add_rebar(&[plus.clone(), invalid]).is_err());
        assert_eq!(p.rebar_rows().unwrap().len(), 2);
        p.db.pragma_update(None, "query_only", true).unwrap();
        assert!(p.add_rebar(&[plus]).is_err());
        p.db.pragma_update(None, "query_only", false).unwrap();
        p.relink_drawing("old-plan.dwg", "saved-plan.dwg").unwrap();
        assert_eq!(p.rebar_rows().unwrap()[0].drawing, "saved-plan.dwg");
        p.save_rebar_settings(&RebarSettings {
            mode: RebarMode::Vertical,
            multiplier: 3,
            symbols: "ΦØøϕ".into(),
            tolerance: 0.1,
            marking: RebarMarking::Mixed,
            color: 6,
            recolor_text: false,
            draw_boxes: true,
        })
        .unwrap();
        drop(q);
        drop(p);
        let p = Project::open_shared(&path, id).unwrap();
        assert_eq!(p.rebar_settings().unwrap().mode, RebarMode::Vertical);
        assert_eq!(p.rebar_settings().unwrap().multiplier, 3);
        assert_eq!(p.rebar_settings().unwrap().marking, RebarMarking::Mixed);
        assert_eq!(p.rebar_settings().unwrap().color, 6);
        assert!(!p.rebar_settings().unwrap().recolor_text);
        assert!(p.rebar_settings().unwrap().draw_boxes);
        let summary = summarize_rebar(&p.rebar_rows().unwrap());
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].length_m, 25.);
        assert!((summary[0].net_kg() - 39.50617283950617).abs() < 1e-10);
        drop(p);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn importing_project_preserves_rebar_and_settings() {
        let source = path("source.sqlite");
        let shared = path("shared.sqlite");
        let p = Project::open(&source, "Source").unwrap();
        p.add_rebar(&[RebarEntry::manual(2, 8, 100., false).unwrap()])
            .unwrap();
        p.save_rebar_settings(&RebarSettings {
            mode: RebarMode::Quantityless,
            marking: RebarMarking::Mixed,
            color: 5,
            draw_boxes: true,
            ..Default::default()
        })
        .unwrap();
        drop(p);
        let existing = Project::create_shared(&shared, "Existing").unwrap();
        let id = Project::import_shared(&shared, &source).unwrap();
        let imported = Project::open_shared(&shared, id).unwrap();
        assert_eq!(imported.rebar_rows().unwrap().len(), 1);
        assert_eq!(
            imported.rebar_settings().unwrap().mode,
            RebarMode::Quantityless
        );
        assert!(existing.rebar_rows().unwrap().is_empty());
        assert_eq!(
            imported.rebar_settings().unwrap().marking,
            RebarMarking::Mixed
        );
        assert_eq!(imported.rebar_settings().unwrap().color, 5);
        assert!(imported.rebar_settings().unwrap().draw_boxes);
        drop(imported);
        drop(existing);
        std::fs::remove_file(source).unwrap();
        std::fs::remove_file(shared).unwrap();
    }
    #[test]
    fn excel_has_numeric_measurements_and_escaped_source_text() {
        use std::io::Read;
        let output = path("export.xlsx");
        let mut row = RebarEntry::manual(12, 16, 250., false).unwrap();
        row.source = "<Donatı & 1>".into();
        export_rebar(&output, &[row]).unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
        let mut xml = String::new();
        zip.by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        assert!(xml.contains("&lt;Donatı &amp; 1&gt;"));
        assert!(xml.contains("<v>30</v>"));
        assert!(xml.contains("Net (ton)"));
        assert!(xml.contains("TOPLAM"));
        assert_eq!(zip.len(), 5);
        drop(zip);
        std::fs::remove_file(output).unwrap();
    }
    #[test]
    fn old_rebar_settings_upgrade_without_losing_records_and_import_with_defaults() {
        let source = path("old.sqlite");
        let target = path("target.sqlite");
        let p = Project::open(&source, "Old").unwrap();
        p.add_rebar(&[RebarEntry::manual(4, 12, 200., false).unwrap()])
            .unwrap();
        p.db.execute_batch("DROP TABLE rebar_settings; CREATE TABLE rebar_settings(project_id INTEGER PRIMARY KEY REFERENCES project(id),mode INTEGER NOT NULL,symbols TEXT NOT NULL,multiplier INTEGER NOT NULL,tolerance REAL NOT NULL); INSERT INTO rebar_settings VALUES(1,3,'ΦØ',2,0.2)").unwrap();
        drop(p);
        let id = Project::import_shared(&target, &source).unwrap();
        let imported = Project::open_shared(&target, id).unwrap();
        assert_eq!(imported.rebar_rows().unwrap()[0].quantity, 4);
        assert_eq!(
            imported.rebar_settings().unwrap().marking,
            RebarMarking::Color
        );
        drop(imported);
        let p = Project::open(&source, "Old").unwrap();
        let settings = p.rebar_settings().unwrap();
        assert_eq!(settings.mode, RebarMode::Vertical);
        assert_eq!(settings.multiplier, 2);
        assert_eq!(settings.symbols, "ΦØ");
        assert_eq!(settings.tolerance, 0.2);
        assert_eq!(settings.marking, RebarMarking::Color);
        assert_eq!(settings.color, 3);
        assert_eq!(p.rebar_rows().unwrap().len(), 1);
        drop(p);
        std::fs::remove_file(source).unwrap();
        std::fs::remove_file(target).unwrap();
    }
}
