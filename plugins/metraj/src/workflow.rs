use crate::store::{Item, Record, Store};
use ocs_plugin_api::dialog::{DialogControl as C, DialogDef, DialogResponse, DialogValue as V};
use ocs_plugin_api::host::{EntityType, HostApi, LayerConfig};
use std::collections::{BTreeMap, HashMap};

pub(crate) fn text(id: &str, label: &str, default: &str) -> C {
    C::Text {
        id: id.into(),
        label: label.into(),
        default: default.into(),
    }
}
pub(crate) fn choice(id: &str, label: &str, options: Vec<String>) -> C {
    C::Choice {
        id: id.into(),
        label: label.into(),
        options,
        default_index: 0,
    }
}
pub(crate) fn values(
    host: &mut dyn HostApi,
    title: &str,
    controls: Vec<C>,
) -> Option<HashMap<String, V>> {
    let mut d = DialogDef::new(title)
        .submit_label("Devam")
        .cancel_label("Vazgeç");
    for c in controls {
        d = d.with_control(c);
    }
    match host.show_dialog(d) {
        DialogResponse::Submitted(v) => Some(v),
        _ => None,
    }
}
pub(crate) fn index(v: &HashMap<String, V>, id: &str) -> Result<usize, String> {
    match v.get(id) {
        Some(V::Choice { index, .. }) => Ok(*index),
        _ => Err(format!("{id} seçilmedi.")),
    }
}
pub(crate) fn string(v: &HashMap<String, V>, id: &str) -> Result<String, String> {
    match v.get(id) {
        Some(V::Text(s)) => Ok(s.trim().to_string()),
        _ => Err(format!("{id} girilmedi.")),
    }
}
pub(crate) fn number(v: &HashMap<String, V>, id: &str) -> Result<f64, String> {
    let n = string(v, id)?
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| format!("{id}: geçerli bir sayı girin."))?;
    if !n.is_finite() {
        return Err(format!("{id}: geçerli bir sayı girin."));
    }
    Ok(n)
}
pub(crate) fn target(
    host: &mut dyn HostApi,
    store: &Store,
) -> Result<Option<(usize, usize)>, String> {
    if let Ok(snapshot) = host.panel_snapshot(crate::PANEL_ID) {
        if snapshot.errors.is_empty() {
            if let Some(V::Choice { index, text }) = snapshot.values.get(crate::ACTIVE_GROUP) {
                if let Some((target, _)) = index
                    .checked_sub(1)
                    .and_then(|i| crate::group_choices(store).into_iter().nth(i))
                    .filter(|(_, label)| label == text)
                {
                    return Ok(Some(target));
                }
            }
        }
    }
    if store.projects.is_empty() {
        return Err("Önce bir proje oluşturun.".into());
    }
    let Some(p) = values(
        host,
        "Proje seç",
        vec![choice(
            "project",
            "Proje",
            store.projects.iter().map(|p| p.name.clone()).collect(),
        )],
    ) else {
        return Ok(None);
    };
    let pi = index(&p, "project")?;
    let p = store.projects.get(pi).ok_or("Proje bulunamadı.")?;
    if p.groups.is_empty() {
        return Err("Önce bu projede grup oluşturun.".into());
    }
    let Some(g) = values(
        host,
        "Grup seç",
        vec![choice(
            "group",
            "Grup",
            p.groups.iter().map(|g| g.name.clone()).collect(),
        )],
    ) else {
        return Ok(None);
    };
    let gi = index(&g, "group")?;
    if gi >= p.groups.len() {
        return Err("Grup bulunamadı.".into());
    }
    Ok(Some((pi, gi)))
}
pub(crate) fn item_index(
    host: &mut dyn HostApi,
    group: &crate::store::Group,
) -> Result<Option<usize>, String> {
    if group.items.is_empty() {
        return Err("Önce bir metraj kalemi oluşturun.".into());
    }
    let Some(v) = values(
        host,
        "Kalem seç",
        vec![choice(
            "item",
            "Kalem",
            group
                .items
                .iter()
                .map(|i| format!("{} ({})", i.name, i.unit_label()))
                .collect(),
        )],
    ) else {
        return Ok(None);
    };
    let i = index(&v, "item")?;
    if i >= group.items.len() {
        return Err("Kalem bulunamadı.".into());
    }
    Ok(Some(i))
}
pub fn create_item(host: &mut dyn HostApi) -> Result<(), String> {
    let mut s = Store::load()?;
    let Some((p, g)) = target(host, &s)? else {
        return Ok(());
    };
    let Some(v) = values(
        host,
        "Metraj kalemi",
        vec![
            text("name", "Kalem adı", ""),
            text("poz", "Poz no", ""),
            choice(
                "unit",
                "Birim",
                ["m", "m²", "m³", "adet", "kg", "ton"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            ),
        ],
    ) else {
        return Ok(());
    };
    let name = string(&v, "name")?;
    if name.is_empty() {
        return Err("Kalem adı boş olamaz.".into());
    }
    let unit = ["m", "m²", "m³", "adet", "kg", "ton"]
        .get(index(&v, "unit")?)
        .ok_or("Birim bulunamadı.")?
        .to_string();
    s.projects[p].groups[g].items.push(Item {
        name,
        poz: string(&v, "poz")?,
        unit,
        display_unit: None,
        records: Vec::new(),
    });
    s.save()?;
    host.push_info("Metraj kalemi oluşturuldu.");
    Ok(())
}
pub fn manual(host: &mut dyn HostApi) -> Result<(), String> {
    let mut s = Store::load()?;
    let Some((p, g)) = target(host, &s)? else {
        return Ok(());
    };
    let Some(i) = item_index(host, &s.projects[p].groups[g])? else {
        return Ok(());
    };
    let Some(v) = values(
        host,
        "Manuel metraj",
        vec![
            text("quantity", "Miktar (minha için negatif)", ""),
            text("note", "Açıklama", ""),
        ],
    ) else {
        return Ok(());
    };
    let q = number(&v, "quantity")?;
    if q == 0.0 {
        return Err("Miktar sıfır olamaz.".into());
    }
    s.projects[p].groups[g].items[i].records.push(Record {
        method: "Manuel".into(),
        quantity: q,
        source: string(&v, "note")?,
        handles: Vec::new(),
        original_layers: Vec::new(),
    });
    s.save()?;
    host.push_info("Manuel metraj kaydedildi.");
    Ok(())
}
fn poly_measure(
    p: &ocs_plugin_api::host::document::entities::LwPolyline,
) -> Result<(f64, f64), String> {
    let v = &p.vertices;
    if v.len() < 2 {
        return Err("Polyline yeterli köşe içermiyor.".into());
    }
    let mut length = 0.0;
    let mut twice_area = 0.0;
    let mut arc_area = 0.0;
    let origin = v[0].location;
    let count = if p.is_closed { v.len() } else { v.len() - 1 };
    for i in 0..count {
        let a = &v[i];
        let b = &v[(i + 1) % v.len()];
        let dx = b.location.x - a.location.x;
        let dy = b.location.y - a.location.y;
        let chord = dx.hypot(dy);
        if chord == 0.0 {
            if a.bulge != 0.0 {
                return Err("Sıfır uzunluklu yay parçası ölçülemez.".into());
            }
            continue;
        }
        twice_area += (a.location.x - origin.x) * (b.location.y - origin.y)
            - (b.location.x - origin.x) * (a.location.y - origin.y);
        if a.bulge.abs() < 1e-12 {
            length += chord;
        } else {
            let theta = 4.0 * a.bulge.atan();
            let radius = chord / (2.0 * (theta / 2.0).sin().abs());
            length += radius * theta.abs();
            arc_area += radius * radius * (theta - theta.sin()) / 2.0;
        }
    }
    Ok((
        length,
        if p.is_closed {
            (twice_area / 2.0 + arc_area).abs()
        } else {
            0.0
        },
    ))
}
pub fn measure(host: &mut dyn HostApi) -> Result<(), String> {
    let selected = host.selection();
    if selected.is_empty() {
        return Err("Önce çizimde ölçülecek nesneleri seçin.".into());
    }
    let mut s = Store::load()?;
    let Some((p, g)) = target(host, &s)? else {
        return Ok(());
    };
    let Some(i) = item_index(host, &s.projects[p].groups[g])? else {
        return Ok(());
    };
    let unit = s.projects[p].groups[g].items[i].unit.clone();
    let modes =
        match unit.as_str() {
            "m" => vec!["Uzunluk"],
            "m²" => vec!["Alan", "Uzunluk × yükseklik"],
            "m³" => vec!["Alan × yükseklik"],
            _ => return Err(
                "Bu birimde çizimden alan/uzunluk ölçümü desteklenmiyor; manuel giriş kullanın."
                    .into(),
            ),
        };
    let Some(v) = values(
        host,
        "Çizimden metraj",
        vec![
            choice(
                "method",
                "Yöntem",
                modes.iter().map(|s| s.to_string()).collect(),
            ),
            choice(
                "scale",
                "Çizim birimi",
                ["cm", "mm", "m"].iter().map(|s| s.to_string()).collect(),
            ),
            text("height", "Yükseklik (m)", "3"),
            choice("sign", "İşlem", vec!["Metraj".into(), "Minha".into()]),
        ],
    ) else {
        return Ok(());
    };
    let method = *modes
        .get(index(&v, "method")?)
        .ok_or("Yöntem bulunamadı.")?;
    let factor = *[0.01, 0.001, 1.0]
        .get(index(&v, "scale")?)
        .ok_or("Birim bulunamadı.")?;
    let h = if method.contains('×') {
        let h = number(&v, "height")?;
        if h <= 0.0 {
            return Err("Yükseklik pozitif olmalıdır.".into());
        }
        h
    } else {
        1.0
    };
    let area = method.starts_with("Alan");
    let mut sum = 0.0;
    let mut originals = Vec::new();
    let mut layers = Vec::new();
    for handle in &selected {
        let entity = host
            .document()
            .get_entity(*handle)
            .ok_or("Seçilen nesne artık bulunmuyor.")?;
        let (len,a)=match entity {EntityType::Line(l) if !area=>(((l.end.x-l.start.x).powi(2)+(l.end.y-l.start.y).powi(2)+(l.end.z-l.start.z).powi(2)).sqrt(),0.0),EntityType::LwPolyline(pl)=>{if area&&!pl.is_closed{return Err("Alan için kapalı polyline seçilmelidir.".into());}poly_measure(pl)?},EntityType::Hatch(h) if area=>(0.0,h.area().ok_or("Hatch alanı hesaplanamadı.")?.abs()),_=>return Err("Seçimde bu yöntemle ölçülemeyen bir nesne var. Uzunluk için LINE/LWPOLYLINE, alan için kapalı LWPOLYLINE veya HATCH seçin.".into())};
        sum += if area {
            a * factor * factor
        } else {
            len * factor
        };
        layers.push(entity.common().layer.clone());
        originals.push(entity.clone());
    }
    let mut quantity = sum * h;
    if index(&v, "sign")? == 1 {
        quantity = -quantity;
    }
    if !quantity.is_finite() || quantity == 0.0 {
        return Err("Ölçüm sonucu sıfır veya geçersiz.".into());
    }
    s.projects[p].groups[g].items[i].records.push(Record {
        method: method.into(),
        quantity,
        source: format!("{} nesne", selected.len()),
        handles: selected.iter().map(|h| h.value()).collect(),
        original_layers: layers,
    });
    s.save()?;
    let calculated = host.document().layers.get("HESAPLANDI").is_some()
        || host
            .add_layer(LayerConfig {
                name: "HESAPLANDI".into(),
                ..Default::default()
            })
            .is_some();
    if calculated {
        for e in &mut originals {
            e.common_mut().layer = "HESAPLANDI".into();
            e.common_mut().color = ocs_plugin_api::host::document::types::Color::ByLayer;
        }
        if let Err(e) = host.update_entities_transaction("Metraj nesnelerini işaretle", originals)
        {
            host.push_error(&format!("Metraj kaydedildi; katman güncellenemedi: {e}"));
        }
    } else {
        host.push_error("Metraj kaydedildi; HESAPLANDI katmanı oluşturulamadı.");
    }
    host.push_info(&format!("Metraj kaydedildi: {:.4} {}", quantity, unit));
    Ok(())
}
pub fn summary(host: &mut dyn HostApi) -> Result<(), String> {
    let s = Store::load()?;
    let Some((p, g)) = target(host, &s)? else {
        return Ok(());
    };
    let group = &s.projects[p].groups[g];
    let mut lines = vec![format!("{} / {}", s.projects[p].name, group.name)];
    let mut totals: BTreeMap<&str, f64> = BTreeMap::new();
    for item in &group.items {
        let total: f64 = item.records.iter().map(|r| r.quantity).sum();
        lines.push(format!(
            "{}  {} — {:.3} {}",
            item.poz,
            item.name,
            total,
            item.unit_label()
        ));
        *totals.entry(item.unit_label()).or_default() += total;
    }
    let mut steel: BTreeMap<u32, (f64, f64)> = BTreeMap::new();
    for r in &group.rebar {
        let total = steel.entry(r.diameter).or_default();
        total.0 +=
            r.quantity as f64 * r.length_cm / 100.0 * if r.weight_kg < 0.0 { -1.0 } else { 1.0 };
        total.1 += r.weight_kg;
    }
    for (d, (length, kg)) in &steel {
        lines.push(format!(
            "Demir Ø{d}: {length:.2} m | {kg:.3} kg | {:.3} ton",
            kg / 1000.0
        ));
    }
    *totals.entry("kg").or_default() += steel.values().map(|(_, kg)| kg).sum::<f64>();
    for (u, n) in totals {
        lines.push(format!("TOPLAM: {n:.3} {u}"));
    }
    host.show_dialog(
        DialogDef::new("Grup İcmali")
            .with_control(C::InfoText {
                text: lines.join("\n"),
                align_h: None,
            })
            .submit_label("Kapat"),
    );
    Ok(())
}
