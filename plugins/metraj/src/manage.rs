use crate::panels::{active_group, active_item, active_project};
use crate::store::{RebarRow, Store};
use crate::workflow::{choice, index, number, string, text, values, UNITS};
use ocs_plugin_api::dialog::{DialogControl as C, DialogDef, DialogResponse, DialogValue as V};
use ocs_plugin_api::host::HostApi;
use rust_xlsxwriter::Workbook;
use std::collections::{BTreeMap, HashMap};

pub fn manual_rebar(
    host: &mut dyn HostApi,
    s: &mut Store,
    v: &HashMap<String, V>,
) -> Result<(), String> {
    let (p, g) = active_group(s)?;
    let quantity = string(v, "quantity")?
        .parse::<u32>()
        .map_err(|_| "Adet pozitif tam sayı olmalı.")?;
    let diameter = string(v, "diameter")?
        .parse::<u32>()
        .map_err(|_| "Çap pozitif tam sayı olmalı.")?;
    let length_cm = number(v, "length")?;
    if quantity == 0 || diameter == 0 || length_cm <= 0.0 {
        return Err("Adet, çap ve boy pozitif olmalı.".into());
    }
    let weight_kg = quantity as f64 * length_cm / 100.0 * (diameter as f64).powi(2) / 162.0
        * if index(v, "sign")? == 1 { -1.0 } else { 1.0 };
    if !weight_kg.is_finite() {
        return Err("Hesap sayısal sınırı aşıyor.".into());
    }
    s.projects[p].groups[g].rebar.push(RebarRow {
        quantity,
        diameter,
        length_cm,
        weight_kg,
        source: "Manuel".into(),
        handles: Vec::new(),
        original_layers: Vec::new(),
    });
    s.save()?;
    host.push_info("Manuel demir kaydedildi.");
    Ok(())
}
pub fn delete_record(host: &mut dyn HostApi, s: &mut Store, rebar: bool) -> Result<(), String> {
    let (p, g) = active_group(s)?;
    let record = crate::panels::record_index(host, rebar)?;
    let group = &mut s.projects[p].groups[g];
    let description = if rebar {
        let r = group.rebar.get(record).ok_or("Demir kaydı bulunamadı.")?;
        format!(
            "{} adet Ø{} · {:.3} kg",
            r.quantity, r.diameter, r.weight_kg
        )
    } else {
        let item = &group.items[active_item(group)?];
        let r = item.records.get(record).ok_or("Metraj kaydı bulunamadı.")?;
        format!("{} · {:.4} {}", item.name, r.quantity, item.unit_label())
    };
    if !confirm(
        host,
        &format!("{} numaralı kayıt silinsin mi?\n{description}", record + 1),
    ) {
        return Ok(());
    }
    if rebar {
        group.rebar.remove(record);
    } else {
        let i = active_item(group)?;
        group.items[i].records.remove(record);
    }
    s.save()?;
    host.push_info("Kayıt silindi.");
    Ok(())
}

pub fn delete_item(host: &mut dyn HostApi, s: &mut Store) -> Result<(), String> {
    let (p, g) = active_group(s)?;
    let group = &mut s.projects[p].groups[g];
    let i = active_item(group)?;
    if !confirm(
        host,
        &format!(
            "{} kalemi ve tüm ölçüm kayıtları silinsin mi?",
            group.items[i].name
        ),
    ) {
        return Ok(());
    }
    group.items.remove(i);
    s.save()?;
    crate::panels::close_item();
    host.push_info("Metraj kalemi silindi.");
    Ok(())
}

fn confirm(host: &mut dyn HostApi, message: &str) -> bool {
    matches!(
        host.show_dialog(
            DialogDef::new("Onay")
                .with_control(C::InfoText {
                    text: message.into(),
                    align_h: None
                })
                .submit_label("Sil")
                .cancel_label("Vazgeç")
        ),
        DialogResponse::Submitted(_)
    )
}
pub fn edit_item(host: &mut dyn HostApi, s: &mut Store) -> Result<(), String> {
    let (p, g) = active_group(s)?;
    let i = active_item(&s.projects[p].groups[g])?;
    let current = &s.projects[p].groups[g].items[i];
    let units = UNITS;
    let selected = units.iter().position(|u| *u == current.unit).unwrap_or(0);
    let Some(v) = values(
        host,
        "Kalem düzenle / çoğalt",
        vec![
            text("name", "Kalem adı", &current.name),
            text("poz", "Poz no", &current.poz),
            C::Choice {
                id: "unit".into(),
                label: "Birim (miktarlar dönüştürülmez)".into(),
                options: units.iter().map(|u| u.to_string()).collect(),
                default_index: selected,
            },
            choice(
                "action",
                "İşlem",
                vec!["Düzenle".into(), "Kayıtlarla çoğalt".into()],
            ),
        ],
    ) else {
        return Ok(());
    };
    let name = string(&v, "name")?;
    if name.is_empty() {
        return Err("Kalem adı boş olamaz.".into());
    }
    let unit = units
        .get(index(&v, "unit")?)
        .ok_or("Birim bulunamadı.")?
        .to_string();
    let poz = string(&v, "poz")?;
    let group = &mut s.projects[p].groups[g];
    if index(&v, "action")? == 1 {
        let mut duplicate = group.items[i].clone();
        duplicate.name = name;
        duplicate.poz = poz;
        if duplicate.unit != unit {
            duplicate.display_unit = None;
        }
        duplicate.unit = unit;
        group.items.push(duplicate);
    } else {
        let item = &mut group.items[i];
        item.name = name;
        item.poz = poz;
        if item.unit != unit {
            item.display_unit = None;
        }
        item.unit = unit;
    }
    s.save()?;
    host.push_info("Kalem kaydedildi.");
    Ok(())
}
pub fn edit(host: &mut dyn HostApi, s: &mut Store) -> Result<(), String> {
    let p = active_project(s)?;
    let group_index = active_group(s).ok().map(|(_, g)| g);
    let mut scopes = vec!["Seçili proje".into()];
    if group_index.is_some() {
        scopes.push("Etkin grup".into());
    }
    let Some(v) = values(
        host,
        "Proje ve grup yönetimi",
        vec![
            choice("scope", "Tür", scopes),
            choice("action", "İşlem", vec!["Ad değiştir".into(), "Sil".into()]),
        ],
    ) else {
        return Ok(());
    };
    let group_index = match index(&v, "scope")? {
        0 => None,
        1 => Some(group_index.ok_or("Etkin grup bulunamadı.")?),
        _ => return Err("Tür bulunamadı.".into()),
    };
    let old = match group_index {
        Some(g) => s.projects[p].groups[g].name.clone(),
        None => s.projects[p].name.clone(),
    };
    if index(&v, "action")? == 1 {
        if !confirm(
            host,
            &format!("{old} ve içerdiği tüm kayıtlar silinsin mi?"),
        ) {
            return Ok(());
        }
        match group_index {
            Some(g) => {
                s.projects[p].groups.remove(g);
            }
            None => {
                s.projects.remove(p);
            }
        }
        s.save()?;
    } else {
        let Some(v) = values(host, "Ad değiştir", vec![text("name", "Yeni ad", &old)]) else {
            return Ok(());
        };
        let name = string(&v, "name")?;
        if name.is_empty() {
            return Err("Ad boş olamaz.".into());
        }
        if let Some(g) = group_index {
            let groups = &mut s.projects[p].groups;
            if groups
                .iter()
                .enumerate()
                .any(|(i, other)| i != g && other.name.eq_ignore_ascii_case(&name))
            {
                return Err("Bu grup adı zaten var.".into());
            }
            groups[g].name = name.clone();
            s.save()?;
            crate::panels::rename_group(&old, name);
        } else {
            if s.projects
                .iter()
                .enumerate()
                .any(|(i, other)| i != p && other.name.eq_ignore_ascii_case(&name))
            {
                return Err("Bu proje adı zaten var.".into());
            }
            s.projects[p].name = name.clone();
            s.save()?;
            crate::panels::rename_project(&old, name);
        }
    }
    host.push_info("Değişiklik kaydedildi.");
    Ok(())
}
pub fn export(host: &mut dyn HostApi, s: &Store) -> Result<(), String> {
    let (p, g) = active_group(s)?;
    let group = &s.projects[p].groups[g];
    let d = DialogDef::new("Grup icmalini Excel'e aktar")
        .with_control(C::FilePicker {
            id: "path".into(),
            label: "Excel dosyası".into(),
            default_path: "Metraj.xlsx".into(),
            filter_name: "Excel".into(),
            extensions: vec!["xlsx".into()],
            save: true,
        })
        .submit_label("Aktar")
        .cancel_label("Vazgeç");
    let DialogResponse::Submitted(v) = host.show_dialog(d) else {
        return Ok(());
    };
    let path = match v.get("path") {
        Some(V::FilePath(x)) if !x.trim().is_empty() => x,
        _ => return Err("Excel dosyası seçilmedi.".into()),
    };
    if !std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xlsx"))
    {
        return Err("Excel dosyasının uzantısı .xlsx olmalıdır.".into());
    }
    let mut wb = Workbook::new();
    let sheet = wb.add_worksheet();
    sheet.set_name("Grup İcmali").map_err(|e| e.to_string())?;
    for (col, title) in ["Poz No", "Kalem", "Miktar", "Birim"].iter().enumerate() {
        sheet
            .write_string(0, col as u16, *title)
            .map_err(|e| e.to_string())?;
    }
    let mut row = 1;
    let mut totals = BTreeMap::<String, f64>::new();
    for item in &group.items {
        let n: f64 = item.records.iter().map(|r| r.quantity).sum();
        sheet
            .write_string(row, 0, &item.poz)
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 1, &item.name)
            .map_err(|e| e.to_string())?;
        sheet.write_number(row, 2, n).map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 3, item.unit_label())
            .map_err(|e| e.to_string())?;
        *totals.entry(item.unit_label().to_owned()).or_default() += n;
        row += 1;
    }
    let mut diameters = BTreeMap::<u32, f64>::new();
    for r in &group.rebar {
        *diameters.entry(r.diameter).or_default() += r.weight_kg;
    }
    for (d, n) in diameters {
        sheet
            .write_string(row, 1, format!("Demir Ø{d}"))
            .map_err(|e| e.to_string())?;
        sheet.write_number(row, 2, n).map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 3, "kg")
            .map_err(|e| e.to_string())?;
        *totals.entry("kg".into()).or_default() += n;
        row += 1;
    }
    for (unit, n) in totals {
        sheet
            .write_string(row, 1, "GENEL TOPLAM")
            .map_err(|e| e.to_string())?;
        sheet.write_number(row, 2, n).map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 3, unit)
            .map_err(|e| e.to_string())?;
        row += 1;
    }
    sheet.set_column_width(0, 18).map_err(|e| e.to_string())?;
    sheet.set_column_width(1, 35).map_err(|e| e.to_string())?;
    sheet.set_column_width(2, 18).map_err(|e| e.to_string())?;
    let sheet = wb.add_worksheet();
    sheet
        .set_name("Metraj Kayıtları")
        .map_err(|e| e.to_string())?;
    for (col, title) in ["Poz No", "Kalem", "Yöntem", "Miktar", "Birim", "Kaynak"]
        .iter()
        .enumerate()
    {
        sheet
            .write_string(0, col as u16, *title)
            .map_err(|e| e.to_string())?;
    }
    let mut row = 1;
    for item in &group.items {
        for record in &item.records {
            sheet
                .write_string(row, 0, &item.poz)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(row, 1, &item.name)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(row, 2, &record.method)
                .map_err(|e| e.to_string())?;
            sheet
                .write_number(row, 3, record.quantity)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(row, 4, item.unit_label())
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(row, 5, &record.source)
                .map_err(|e| e.to_string())?;
            row += 1;
        }
    }
    let sheet = wb.add_worksheet();
    sheet.set_name("Demir Metrajı").map_err(|e| e.to_string())?;
    for (col, title) in [
        "Adet",
        "Çap (mm)",
        "Birim boy (m)",
        "Toplam boy (m)",
        "Birim ağırlık (kg/m)",
        "Toplam ağırlık (kg)",
        "Kaynak",
    ]
    .iter()
    .enumerate()
    {
        sheet
            .write_string(0, col as u16, *title)
            .map_err(|e| e.to_string())?;
    }
    for (i, r) in group.rebar.iter().enumerate() {
        let row = (i + 1) as u32;
        let sign = if r.weight_kg < 0.0 { -1.0 } else { 1.0 };
        let values = [
            r.quantity as f64,
            r.diameter as f64,
            r.length_cm / 100.0,
            r.quantity as f64 * r.length_cm / 100.0 * sign,
            (r.diameter as f64).powi(2) / 162.0,
            r.weight_kg,
        ];
        for (col, n) in values.iter().enumerate() {
            sheet
                .write_number(row, col as u16, *n)
                .map_err(|e| e.to_string())?;
        }
        sheet
            .write_string(row, 6, &r.source)
            .map_err(|e| e.to_string())?;
    }
    wb.save(path)
        .map_err(|e| format!("Excel dosyası kaydedilemedi: {e}"))?;
    host.push_info("Grup icmali Excel'e aktarıldı.");
    Ok(())
}
