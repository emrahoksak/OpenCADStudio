use crate::store::{RebarRow, RebarSettings, Store};
use crate::workflow::{choice, index, number, string, text};
use document::entities::Entity;
use document::types::{BoundingBox3D, Color, Vector2, Vector3};
use ocs_plugin_api::dialog::{DialogControl as C, DialogValue as V};
use ocs_plugin_api::host::{document, EntityType, HostApi, LayerConfig};
use regex::Regex;
use std::collections::HashMap;

const MODES: [&str; 6] = ["Mod1", "Mod2", "Yatay", "Düşey", "Adetsiz", "Mod2++"];

pub(crate) fn mode_label(mode: usize) -> &'static str {
    MODES.get(mode).copied().unwrap_or(MODES[0])
}

const COLORS: [(&str, [u8; 3]); 7] = [
    ("Kırmızı", [255, 64, 64]),
    ("Turuncu", [255, 153, 0]),
    ("Sarı", [255, 221, 0]),
    ("Yeşil", [0, 200, 0]),
    ("Camgöbeği", [0, 204, 221]),
    ("Mavi", [64, 128, 255]),
    ("Mor", [204, 102, 255]),
];

fn hex_color(rgb: [u8; 3]) -> String {
    let [r, g, b] = rgb;
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub(crate) fn settings_controls(old: &RebarSettings) -> Vec<C> {
    vec![
        C::Choice {
            id: "mode".into(),
            label: "Eşleştirme modu".into(),
            options: MODES.iter().map(|s| s.to_string()).collect(),
            default_index: old.mode.min(MODES.len() - 1),
        },
        text("symbols", "Çap sembolleri", &old.symbols),
        text(
            "tolerance",
            "Yatay/düşey tolerans",
            &old.tolerance.to_string(),
        ),
        C::Checkbox {
            id: "mixed".into(),
            label: "Her eşleşmeye ayrı renk ver".into(),
            default: old.mixed,
        },
        C::row(vec![
            choice(
                "color_preset",
                "Tek renk",
                std::iter::once("Özel renk".into())
                    .chain(COLORS.iter().map(|(name, _)| (*name).into()))
                    .collect(),
            ),
            text("color", "Özel renk (#RRGGBB)", &hex_color(old.color)),
        ]),
        C::Checkbox {
            id: "box".into(),
            label: "Eşleşmelere çerçeve çiz".into(),
            default: old.bounding_box,
        },
    ]
}

pub(crate) fn settings_values(old: &RebarSettings) -> HashMap<String, V> {
    [
        (
            "mode".into(),
            V::Choice {
                index: old.mode.min(MODES.len() - 1),
                text: mode_label(old.mode).into(),
            },
        ),
        ("symbols".into(), V::Text(old.symbols.clone())),
        ("tolerance".into(), V::Text(old.tolerance.to_string())),
        ("mixed".into(), V::Bool(old.mixed)),
        ("color".into(), V::Text(hex_color(old.color))),
        (
            "color_preset".into(),
            V::Choice {
                index: 0,
                text: "Özel renk".into(),
            },
        ),
        ("box".into(), V::Bool(old.bounding_box)),
    ]
    .into()
}

pub fn save_settings(store: &mut Store, p: usize, v: &HashMap<String, V>) -> Result<(), String> {
    let tol = number(v, "tolerance")?;
    if tol < 0.0 {
        return Err("Tolerans negatif olamaz.".into());
    }
    let symbols: String = string(v, "symbols")?
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '|')
        .collect();
    if symbols.is_empty() {
        return Err("En az bir çap sembolü girin.".into());
    }
    let setting = &mut store.projects[p].settings;
    setting.mode = index(v, "mode")?;
    if setting.mode >= MODES.len() {
        return Err("Mod geçersiz.".into());
    }
    setting.symbols = symbols;
    setting.tolerance = tol;
    setting.mixed = matches!(v.get("mixed"), Some(V::Bool(true)));
    setting.color = match index(v, "color_preset")? {
        0 => parse_color(&string(v, "color")?)?,
        n => COLORS.get(n - 1).ok_or("Renk bulunamadı.")?.1,
    };
    setting.bounding_box = matches!(v.get("box"), Some(V::Bool(true)));
    store.save()
}

fn parse_color(text: &str) -> Result<[u8; 3], String> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Özel renk için #RRGGBB biçiminde bir değer girin.".into());
    }
    let rgb = u32::from_str_radix(hex, 16).map_err(|err| err.to_string())?;
    let [_, r, g, b] = rgb.to_be_bytes();
    Ok([r, g, b])
}

struct TextInfo {
    entity: EntityType,
    text: String,
    position: [f64; 3],
    quantity: u32,
    diameter: u32,
    length: f64,
}
fn quantity(expr: &str) -> Option<u32> {
    let mut result = 1u32;
    for factor in expr.split(|c: char| matches!(c, 'x' | 'X' | '×' | '*')) {
        let n = factor.trim().parse::<u32>().ok()?;
        if n == 0 {
            return None;
        }
        result = result.checked_mul(n)?;
    }
    Some(result)
}
fn cap(value: &str, symbols: &str) -> Option<(u32, u32)> {
    let q = Regex::new(r"(?i)(\d+(?:\s*[x×*]\s*\d+)*)\s*$").ok()?;
    let d = Regex::new(r"^\s*(\d+)").ok()?;
    let mut found = None;
    for (i, c) in value.char_indices() {
        if !symbols.contains(c) {
            continue;
        }
        let before = &value[..i];
        if let Some(m) = q.find(before) {
            let prefix = before[..m.start()].trim_end();
            if prefix.ends_with(|c: char| "xX×*.,-+".contains(c)) {
                continue;
            }
            if let (Some(qty), Some(dia)) = (
                quantity(m.as_str()),
                d.captures(&value[i + c.len_utf8()..])
                    .and_then(|v| v[1].parse::<u32>().ok()),
            ) {
                if (1..=100).contains(&dia) {
                    found = Some((qty, dia));
                }
            }
        }
    }
    found
}
fn length(value: &str) -> Option<f64> {
    let re = Regex::new(r"(?i)\bL\s*=\s*(\d+(?:[.,]\d+)?)").ok()?;
    let n = re.captures(value)?[1]
        .replace(',', ".")
        .parse::<f64>()
        .ok()?;
    if n.is_finite() && n > 0.0 {
        Some(n)
    } else {
        None
    }
}
fn diameter(value: &str, symbols: &str) -> Option<u32> {
    let v = value.trim_start();
    let c = v.chars().next()?;
    if !symbols.contains(c) {
        return None;
    }
    let re = Regex::new(r"^\s*(\d+)").ok()?;
    let d = re.captures(&v[c.len_utf8()..])?[1].parse::<u32>().ok()?;
    if (1..=100).contains(&d) {
        Some(d)
    } else {
        None
    }
}
fn quantity_label(value: &str) -> Option<u32> {
    let re = Regex::new(r"(?i)(\d+(?:\s*[x×*]\s*\d+)*)\s*A(?:D|T)ET\b").ok()?;
    let captures = re.captures_iter(value).last()?;
    let quantity_match = captures.get(1)?;
    let prefix = value[..quantity_match.start()].trim_end();
    if prefix.ends_with(|c: char| "xX×*.,-+".contains(c)) {
        return None;
    }
    quantity(quantity_match.as_str())
}
fn rgb(index: usize) -> [u8; 3] {
    let h = (index as f64 * 137.508) % 360.0;
    let chroma = 0.95 * 0.78;
    let x = chroma * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = if h < 60.0 {
        (chroma, x, 0.0)
    } else if h < 120.0 {
        (x, chroma, 0.0)
    } else if h < 180.0 {
        (0.0, chroma, x)
    } else if h < 240.0 {
        (0.0, x, chroma)
    } else if h < 300.0 {
        (x, 0.0, chroma)
    } else {
        (chroma, 0.0, x)
    };
    let offset = 0.95 - chroma;
    [
        ((r + offset) * 255.0).round() as u8,
        ((g + offset) * 255.0).round() as u8,
        ((b + offset) * 255.0).round() as u8,
    ]
}

fn mtext_bounds(text: &document::entities::MText) -> BoundingBox3D {
    let plain = document::entities::mtext_format::parse_mtext(&text.value, true).to_plain_text();
    let width = if text.rectangle_width > 0.0 {
        text.rectangle_width
    } else {
        plain
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(1) as f64
            * text.height
            * 0.6
    };
    let height = text
        .rectangle_height
        .filter(|height| *height > 0.0)
        .unwrap_or_else(|| {
            text.height
                * (1.0
                    + plain.lines().count().saturating_sub(1) as f64 * 5.0 / 3.0
                        * text.line_spacing_factor)
        });
    let attachment = text.attachment_point as u8 - 1;
    let left = -width * (attachment % 3) as f64 / 2.0;
    let top = height * (attachment / 3) as f64 / 2.0;
    let origin = text.insertion_point;
    let (sin, cos) = text.rotation.sin_cos();
    let mut min = Vector3::new(f64::INFINITY, f64::INFINITY, origin.z);
    let mut max = Vector3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, origin.z);
    for (x, y) in [
        (left, top),
        (left + width, top),
        (left + width, top - height),
        (left, top - height),
    ] {
        let x_world = origin.x + x * cos - y * sin;
        let y_world = origin.y + x * sin + y * cos;
        min.x = min.x.min(x_world);
        min.y = min.y.min(y_world);
        max.x = max.x.max(x_world);
        max.y = max.y.max(y_world);
    }
    BoundingBox3D::new(min, max)
}

pub fn capture(
    host: &mut dyn HostApi,
    store: &mut Store,
    v: &HashMap<String, V>,
) -> Result<(), String> {
    let handles = host.selection();
    if handles.is_empty() {
        return Err("Önce demir TEXT/MTEXT nesnelerini seçin.".into());
    }
    let (p, g) = crate::panels::active_group(store)?;
    let mult = string(v, "multiplier")?
        .parse::<u32>()
        .map_err(|_| "x adet pozitif tam sayı olmalı.")?;
    if mult == 0 {
        return Err("x adet pozitif olmalı.".into());
    }
    let sign = if index(v, "sign")? == 1 { -1.0 } else { 1.0 };
    let settings = &store.projects[p].settings;
    let mut texts = Vec::new();
    for handle in handles {
        let entity = host
            .document()
            .get_entity(handle)
            .ok_or("Seçilen nesne bulunamadı.")?
            .clone();
        let (raw, pos) = match &entity {
            EntityType::Text(t) => (
                t.value.clone(),
                [
                    t.insertion_point.x,
                    t.insertion_point.y,
                    t.insertion_point.z,
                ],
            ),
            EntityType::MText(t) => (
                document::entities::mtext_format::parse_mtext(&t.value, true).to_plain_text(),
                [
                    t.insertion_point.x,
                    t.insertion_point.y,
                    t.insertion_point.z,
                ],
            ),
            _ => return Err("Yalnızca TEXT/MTEXT seçin.".into()),
        };
        texts.push(TextInfo {
            entity,
            text: raw
                .replace("%%c", "Ø")
                .replace("%%C", "Ø")
                .replace("\\P", " ")
                .replace("\\p", " ")
                .replace(['\n', '\r', '\t'], " "),
            position: pos,
            quantity: 0,
            diameter: 0,
            length: 0.0,
        });
    }
    let mut groups: Vec<(Vec<usize>, u32, u32, f64)> = Vec::new();
    if settings.mode == 0 {
        for (i, t) in texts.iter().enumerate() {
            if let (Some((q, d)), Some(l)) = (cap(&t.text, &settings.symbols), length(&t.text)) {
                groups.push((vec![i], q, d, l));
            }
        }
    } else if settings.mode == 4 {
        let mut caps = Vec::new();
        let mut lens = Vec::new();
        let mut quantities = Vec::new();
        for (i, t) in texts.iter().enumerate() {
            if let Some(d) = diameter(&t.text, &settings.symbols) {
                caps.push((i, d));
            } else if let Some(l) = length(&t.text) {
                lens.push((i, l));
            } else if let Some(q) = quantity_label(&t.text) {
                quantities.push((i, q));
            }
        }
        if caps.len() != 1 || lens.len() != 1 {
            return Err(format!(
                "Adetsiz modunda tam bir çap ve bir boy seçin (bulunan: {} çap, {} boy).",
                caps.len(),
                lens.len()
            ));
        }
        let mut ids = vec![caps[0].0, lens[0].0];
        let mut q = if quantities.is_empty() { 1u32 } else { 0 };
        for (i, n) in quantities {
            q = q.checked_add(n).ok_or("Adet sınırı aşıldı.")?;
            ids.push(i);
        }
        groups.push((ids, q, caps[0].1, lens[0].1));
    } else {
        let mut caps = Vec::new();
        let mut lens = Vec::new();
        for (i, t) in texts.iter_mut().enumerate() {
            if let Some((q, d)) = cap(&t.text, &settings.symbols) {
                t.quantity = q;
                t.diameter = d;
                caps.push(i);
            } else if let Some(l) = length(&t.text) {
                t.length = l;
                lens.push(i);
            }
        }
        if settings.mode == 5 {
            if caps.is_empty() || lens.len() != 1 {
                return Err("Mod2++ için en az bir adet/çap ve tam bir L= metni seçin.".into());
            }
            let diameter = texts[caps[0]].diameter;
            let mut quantity = 0u32;
            for &id in &caps {
                if texts[id].diameter != diameter {
                    return Err("Mod2++ için tüm çaplar aynı olmalıdır.".into());
                }
                quantity = quantity
                    .checked_add(texts[id].quantity)
                    .ok_or("Adet sınırı aşıldı.")?;
            }
            caps.push(lens[0]);
            groups.push((caps, quantity, diameter, texts[lens[0]].length));
            caps = Vec::new();
        }
        let mut used = std::collections::HashSet::new();
        for ci in caps {
            let a = &texts[ci];
            let mut best = None;
            for &li in &lens {
                if used.contains(&li) {
                    continue;
                }
                let b = &texts[li];
                let dx = b.position[0] - a.position[0];
                let dy = b.position[1] - a.position[1];
                let dz = b.position[2] - a.position[2];
                let score = match settings.mode {
                    1 => (dx * dx + dy * dy + dz * dz).sqrt(),
                    2 if dy.abs() <= settings.tolerance && dx > 0.0001 => dx,
                    3 if dx.abs() <= settings.tolerance && dy > 0.0001 => dy,
                    _ => continue,
                };
                if best.map_or(true, |(_, s)| score < s) {
                    best = Some((li, score));
                }
            }
            if let Some((li, _)) = best {
                used.insert(li);
                groups.push((vec![ci, li], a.quantity, a.diameter, texts[li].length));
            } else {
                host.push_info(&format!("Eşleştirilemedi: {}", a.text));
            }
        }
    }
    if groups.is_empty() {
        return Err("Geçerli veya eşleşen demir metni bulunamadı.".into());
    }
    let mut rows = Vec::new();
    for (ids, q, d, l) in &groups {
        let qty = q.checked_mul(mult).ok_or("Adet sınırı aşıldı.")?;
        let weight = qty as f64 * l / 100.0 * (*d as f64).powi(2) / 162.0 * sign;
        if !weight.is_finite() {
            return Err("Demir hesabı sayısal sınırı aşıyor.".into());
        }
        rows.push(RebarRow {
            quantity: qty,
            diameter: *d,
            length_cm: *l,
            weight_kg: weight,
            source: ids
                .iter()
                .map(|i| texts[*i].text.as_str())
                .collect::<Vec<_>>()
                .join(" | "),
            handles: ids
                .iter()
                .map(|i| texts[*i].entity.common().handle.value())
                .collect(),
            original_layers: ids
                .iter()
                .map(|i| texts[*i].entity.common().layer.clone())
                .collect(),
        });
    }
    let mixed = settings.mixed;
    let selected_color = settings.color;
    let boxes = settings.bounding_box;
    let color_offset = store.projects[p].groups[g].rebar.len();
    store.projects[p].groups[g].rebar.extend(rows);
    store.save()?;
    host.begin_undo_group("Demir metrajı işaretleri");
    let calculated = if host.document().layers.get("HESAPLANDI").is_some() {
        true
    } else {
        host.add_layer(LayerConfig {
            name: "HESAPLANDI".into(),
            ..Default::default()
        })
        .is_some()
    };
    let box_layer = boxes
        && (host.document().layers.get("METRAJ_ESLESME_KUTU").is_some()
            || host
                .add_layer(LayerConfig {
                    name: "METRAJ_ESLESME_KUTU".into(),
                    ..Default::default()
                })
                .is_some());
    let mut updates = Vec::new();
    let mut rectangles = Vec::new();
    for (i, (ids, _, _, _)) in groups.iter().enumerate() {
        let c = if mixed {
            rgb(color_offset + i)
        } else {
            selected_color
        };
        let color = Color::Rgb {
            r: c[0],
            g: c[1],
            b: c[2],
        };
        let mut bounds = None::<[f64; 5]>;
        for id in ids {
            let mut entity = texts[*id].entity.clone();
            entity.common_mut().color = color;
            if calculated {
                entity.common_mut().layer = "HESAPLANDI".into();
            }
            let b = match &entity {
                EntityType::Text(t) => t.bounding_box(),
                EntityType::MText(t) => mtext_bounds(t),
                _ => unreachable!(),
            };
            bounds = Some(match bounds {
                None => [b.min.x, b.min.y, b.max.x, b.max.y, b.min.z],
                Some(a) => [
                    a[0].min(b.min.x),
                    a[1].min(b.min.y),
                    a[2].max(b.max.x),
                    a[3].max(b.max.y),
                    a[4].min(b.min.z),
                ],
            });
            updates.push(entity);
        }
        if box_layer {
            if let Some(b) = bounds {
                if b[0] < b[2] && b[1] < b[3] {
                    let mut pl = document::entities::LwPolyline::from_points(vec![
                        Vector2::new(b[0], b[1]),
                        Vector2::new(b[2], b[1]),
                        Vector2::new(b[2], b[3]),
                        Vector2::new(b[0], b[3]),
                    ]);
                    pl.is_closed = true;
                    pl.elevation = b[4];
                    pl.common.color = color;
                    pl.common.layer = "METRAJ_ESLESME_KUTU".into();
                    rectangles.push(EntityType::LwPolyline(pl));
                }
            }
        }
    }
    if !calculated {
        host.push_error("Metraj kaydedildi; HESAPLANDI katmanı oluşturulamadı.");
    }
    if boxes && !box_layer {
        host.push_error("Metraj kaydedildi; eşleşme kutusu katmanı oluşturulamadı.");
    }
    if let Err(error) = host.update_entities_transaction("Metraj eşleşmelerini renklendir", updates)
    {
        host.push_error(&format!(
            "Metraj kaydedildi; çizim renklendirilemedi: {error}"
        ));
    } else if !rectangles.is_empty() {
        let expected = rectangles.len();
        let added = host.add_entities(rectangles).len();
        if added != expected {
            host.push_error(&format!(
                "{expected} eşleşme çerçevesinden {added} tanesi çizildi."
            ));
        }
    }
    host.end_undo_group();
    host.push_info(&format!("{} demir satırı kaydedildi.", groups.len()));
    Ok(())
}
