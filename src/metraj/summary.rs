use super::{Project, Unit};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::Path,
};

#[derive(Debug, Clone, Default)]
pub struct SummaryRow {
    pub poz: String,
    pub description: String,
    pub units: BTreeSet<String>,
    pub added: f64,
    pub deducted: f64,
    pub invalid: usize,
}
impl SummaryRow {
    pub fn net(&self) -> f64 {
        self.added - self.deducted
    }
    pub fn unit(&self) -> String {
        self.units.iter().cloned().collect::<Vec<_>>().join(" / ")
    }
}
#[derive(Debug, Default)]
pub struct Summary {
    pub rows: Vec<SummaryRow>,
    pub without_poz: usize,
}
impl Project {
    pub fn summary(&self) -> rusqlite::Result<Summary> {
        let mut rows: BTreeMap<String, SummaryRow> = BTreeMap::new();
        let mut without_poz = 0;
        for group in self.groups()? {
            for floor in self.floors(group.id)? {
                for room in self.rooms(floor.id)? {
                    for entry in self.room_assignments(room.id)? {
                        let poz = entry.poz.trim().to_owned();
                        if poz.is_empty() {
                            without_poz += 1;
                            continue;
                        }
                        let row = rows.entry(poz.clone()).or_insert_with(|| SummaryRow {
                            poz,
                            description: entry.item_name.clone(),
                            ..Default::default()
                        });
                        row.units.insert(
                            match entry.unit {
                                Unit::SquareMeter => "m²",
                                Unit::Meter => "m",
                                Unit::Each => "adet",
                            }
                            .into(),
                        );
                        if !entry.source_valid {
                            row.invalid += 1;
                            continue;
                        }
                        if entry.is_deduction {
                            row.deducted -= entry.net;
                        } else {
                            row.added += entry.net;
                        }
                    }
                }
            }
        }
        Ok(Summary {
            rows: rows.into_values().collect(),
            without_poz,
        })
    }
}
pub(super) fn xml(value: &str) -> String {
    value
        .chars()
        .filter(|c| *c >= ' ' || matches!(c, '\n' | '\r' | '\t'))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn export_summary(path: &Path, summary: &Summary) -> Result<(), String> {
    let mut sheet=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" state=\"frozen\"/></sheetView></sheetViews><cols><col min=\"1\" max=\"1\" width=\"22\" customWidth=\"1\"/><col min=\"2\" max=\"2\" width=\"45\" customWidth=\"1\"/><col min=\"3\" max=\"8\" width=\"18\" customWidth=\"1\"/></cols><sheetData><row r=\"1\">");
    for (i, title) in [
        "Poz no",
        "Tanım",
        "Birim",
        "Ekle",
        "Minha",
        "Net miktar",
        "Geçersiz kaynak",
        "Birim kontrolü",
    ]
    .into_iter()
    .enumerate()
    {
        sheet.push_str(&format!(
            "<c r=\"{}1\" t=\"inlineStr\"><is><t>{}</t></is></c>",
            (b'A' + i as u8) as char,
            xml(title)
        ));
    }
    sheet.push_str("</row>");
    for (i, row) in summary.rows.iter().enumerate() {
        let n = i + 2;
        sheet.push_str(&format!("<row r=\"{n}\">"));
        for (col, value) in [
            ('A', row.poz.clone()),
            ('B', row.description.clone()),
            ('C', row.unit()),
        ] {
            sheet.push_str(&format!(
                "<c r=\"{col}{n}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                xml(&value)
            ));
        }
        for (col, value) in [
            ('D', row.added),
            ('E', row.deducted),
            ('F', row.net()),
            ('G', row.invalid as f64),
        ] {
            sheet.push_str(&format!("<c r=\"{col}{n}\"><v>{value}</v></c>"));
        }
        sheet.push_str(&format!(
            "<c r=\"H{n}\" t=\"inlineStr\"><is><t>{}</t></is></c></row>",
            if row.units.len() > 1 {
                "Karma birim: toplamı kontrol edin"
            } else {
                ""
            }
        ));
    }
    sheet.push_str(&format!(
        "</sheetData><autoFilter ref=\"A1:H{}\"/></worksheet>",
        summary.rows.len() + 1
    ));
    write_sheet(path, sheet)
}
pub(super) fn write_sheet(path: &Path, sheet: String) -> Result<(), String> {
    use zip::{write::SimpleFileOptions, ZipWriter};
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let entries=[
        ("[Content_Types].xml",String::from("<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/></Types>")),
        ("_rels/.rels",String::from("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>")),
        ("xl/workbook.xml",String::from("<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"İcmal\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>")),
        ("xl/_rels/workbook.xml.rels",String::from("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>")),
        ("xl/worksheets/sheet1.xml",sheet),
    ];
    for (name, value) in entries {
        zip.start_file(name, SimpleFileOptions::default())
            .map_err(|e| e.to_string())?;
        zip.write_all(value.as_bytes()).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secondary_label_selects_the_same_room() {
        let p = Project::open(":memory:", "Test").unwrap();
        let g = p.add_group("G").unwrap();
        let f = p.add_floor(g, "F").unwrap();
        let room = p
            .add_bound_room(
                f,
                "Oda",
                "plan.dwg",
                "A0",
                crate::metraj::Measurement {
                    area_m2: 12.0,
                    perimeter_m: 14.0,
                },
            )
            .unwrap();
        p.set_label_handle(room, "A1").unwrap();
        p.set_room_poz_label(room, "A2").unwrap();
        assert_eq!(p.room_for_entity("plan.dwg", "A1").unwrap(), Some(room));
        assert_eq!(p.room_for_entity("plan.dwg", "A2").unwrap(), Some(room));
    }
    use crate::metraj::{QuantityMode, RoomSection};
    #[test]
    fn same_poz_merges_different_items_rooms_and_deductions() {
        let p = Project::open(":memory:", "Test").unwrap();
        let g = p.add_group("İnşaat").unwrap();
        let f = p.add_floor(g, "Zemin").unwrap();
        let a = p.add_room(f, "A").unwrap();
        let b = p.add_room(f, "B").unwrap();
        let x = p.add_item("Seramik", Unit::SquareMeter, "001", "").unwrap();
        let y = p
            .add_item("Başka ad", Unit::SquareMeter, "001", "")
            .unwrap();
        p.assign(a, RoomSection::Floor, x, QuantityMode::Manual(10.0))
            .unwrap();
        p.assign(b, RoomSection::Wall, y, QuantityMode::Manual(7.0))
            .unwrap();
        p.assign_with_deduction(b, RoomSection::Wall, x, QuantityMode::Manual(2.0), true)
            .unwrap();
        let s = p.summary().unwrap();
        assert_eq!(s.rows.len(), 1);
        assert_eq!(s.rows[0].added, 17.0);
        assert_eq!(s.rows[0].deducted, 2.0);
        assert_eq!(s.rows[0].net(), 15.0);
        assert_eq!(s.rows[0].poz, "001");
    }
    #[test]
    fn excel_contains_numeric_totals_and_text_poz() {
        let path = std::env::temp_dir().join(format!("ocs-icmal-{}.xlsx", std::process::id()));
        let s = Summary {
            rows: vec![SummaryRow {
                poz: "001".into(),
                description: "A&B".into(),
                added: 12.0,
                ..Default::default()
            }],
            without_poz: 0,
        };
        export_summary(&path, &s).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        use std::io::Read;
        let mut sheet = String::new();
        z.by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet)
            .unwrap();
        assert!(sheet.contains("<t xml:space=\"preserve\">001</t>"));
        assert!(sheet.contains("A&amp;B"));
        assert!(sheet.contains("<c r=\"F2\"><v>12</v>"));
        drop(z);
        std::fs::remove_file(path).unwrap();
    }
}
