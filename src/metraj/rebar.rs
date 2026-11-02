use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RebarMode {
    #[default]
    Mod1,
    Mod2,
    Horizontal,
    Vertical,
    Quantityless,
}
impl RebarMode {
    pub const ALL: [Self; 5] = [
        Self::Mod1,
        Self::Mod2,
        Self::Horizontal,
        Self::Vertical,
        Self::Quantityless,
    ];
    pub fn command(self) -> &'static str {
        match self {
            Self::Mod1 => "DM1",
            Self::Mod2 => "DM2",
            Self::Horizontal => "DMY",
            Self::Vertical => "DMD",
            Self::Quantityless => "DMA",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Mod1 => "Mod1 · Tek metin",
            Self::Mod2 => "Mod2 · En yakın",
            Self::Horizontal => "Yatay · Sağdaki boy",
            Self::Vertical => "Düşey · Üstteki boy",
            Self::Quantityless => "Adetsiz · Ayrı adet",
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            Self::Mod1 => "12Ø16 L=250 — tüm bilgiler tek yazıda.",
            Self::Mod2 => "12Ø16 ve L=250 — en yakın kullanılmamış boy.",
            Self::Horizontal => "Adet/çapın sağında, aynı Y hizasındaki boy.",
            Self::Vertical => "Adet/çapın üstünde, aynı X hizasındaki boy.",
            Self::Quantityless => "Bir Ø16, bir L=250; isteğe bağlı 12 Adet yazıları.",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RebarSettings {
    pub mode: RebarMode,
    pub symbols: String,
    pub multiplier: i32,
    pub tolerance: f64,
    pub marking: RebarMarking,
    pub color: u8,
    pub recolor_text: bool,
    pub draw_boxes: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RebarMarking {
    #[default]
    Color,
    Mixed,
}
impl RebarMarking {
    pub const ALL: [Self; 2] = [Self::Color, Self::Mixed];
    pub fn label(self) -> &'static str {
        match self {
            Self::Color => "Tek renk",
            Self::Mixed => "Karma renk · Her eşleşme farklı",
        }
    }
}
impl Default for RebarSettings {
    fn default() -> Self {
        Self {
            mode: RebarMode::Mod1,
            symbols: "ΦØøϕ∅⌀".into(),
            multiplier: 1,
            tolerance: 0.01,
            marking: RebarMarking::Color,
            color: 3,
            recolor_text: true,
            draw_boxes: false,
        }
    }
}
impl RebarSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.color == 0 {
            return Err("İşaretleme rengi 1–255 arasında ACI rengi olmalı.".into());
        }
        if self.multiplier < 1 {
            return Err("x adet pozitif tam sayı olmalı.".into());
        }
        if !self.tolerance.is_finite() || self.tolerance < 0. {
            return Err("Tolerans sıfır veya pozitif olmalı.".into());
        }
        if !self.symbols.chars().any(|c| !c.is_whitespace() && c != '|')
            || self
                .symbols
                .chars()
                .any(|c| c.is_ascii_digit() || matches!(c, 'x' | 'X' | '×' | '*' | '=' | '.' | ','))
        {
            return Err("Geçerli çap sembolleri girin; örnek: ΦØøϕ".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RebarText {
    pub handle: String,
    pub text: String,
    pub position: [f64; 3],
    pub layer: String,
}

#[derive(Debug, Clone)]
pub struct RebarEntry {
    pub id: i64,
    pub quantity: i32,
    pub diameter_mm: i32,
    pub length_cm: f64,
    pub deduction: bool,
    pub source: String,
    pub drawing: String,
    pub handles: String,
    pub layer: String,
}
impl RebarEntry {
    pub fn manual(
        quantity: i32,
        diameter_mm: i32,
        length_cm: f64,
        deduction: bool,
    ) -> Result<Self, String> {
        let row = Self {
            id: 0,
            quantity,
            diameter_mm,
            length_cm,
            deduction,
            source: "Manuel giriş".into(),
            drawing: String::new(),
            handles: String::new(),
            layer: String::new(),
        };
        row.validate()?;
        Ok(row)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.quantity < 1
            || !(1..=100).contains(&self.diameter_mm)
            || !self.length_cm.is_finite()
            || self.length_cm <= 0.
            || !self.weight_kg().is_finite()
        {
            return Err(
                "Adet pozitif tam sayı, çap 1–100 mm, boy pozitif cm olmalı; hesap sonlu olmalı."
                    .into(),
            );
        }
        Ok(())
    }
    pub fn total_length_m(&self) -> f64 {
        self.quantity as f64 * self.length_cm / 100. * if self.deduction { -1. } else { 1. }
    }
    pub fn weight_kg(&self) -> f64 {
        self.total_length_m() * (self.diameter_mm as f64).powi(2) / 162.
    }
}

#[derive(Debug, Clone, Default)]
pub struct RebarSummary {
    pub diameter_mm: i32,
    pub length_m: f64,
    pub added_kg: f64,
    pub deducted_kg: f64,
}
impl RebarSummary {
    pub fn net_kg(&self) -> f64 {
        self.added_kg - self.deducted_kg
    }
}
pub fn summarize_rebar(rows: &[RebarEntry]) -> Vec<RebarSummary> {
    let mut result = BTreeMap::<i32, RebarSummary>::new();
    for entry in rows {
        let row = result
            .entry(entry.diameter_mm)
            .or_insert_with(|| RebarSummary {
                diameter_mm: entry.diameter_mm,
                ..Default::default()
            });
        row.length_m += entry.total_length_m();
        if entry.deduction {
            row.deducted_kg -= entry.weight_kg();
        } else {
            row.added_kg += entry.weight_kg();
        }
    }
    result.into_values().collect()
}

fn normalize(text: &str) -> String {
    text.replace("\\P", " ")
        .replace("\\p", " ")
        .replace("%%c", "Ø")
        .replace("%%C", "Ø")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn quantity_suffix(text: &str) -> Option<i32> {
    let mut rest = text.trim_end();
    let mut product = 1i32;
    loop {
        let start = rest.trim_end_matches(|c: char| c.is_ascii_digit()).len();
        if start == rest.len() {
            return None;
        }
        let factor: i32 = rest[start..].parse().ok()?;
        if factor < 1 {
            return None;
        }
        product = product.checked_mul(factor)?;
        rest = rest[..start].trim_end();
        if let Some(last) = rest.chars().last() {
            if "xX×*".contains(last) {
                rest = rest[..rest.len() - last.len_utf8()].trim_end();
                continue;
            }
            if ".,-+".contains(last) {
                return None;
            }
        }
        return Some(product);
    }
}
fn diameter(text: &str, symbols: &str, quantityless: bool) -> Option<(i32, i32)> {
    let mut found = None;
    for (index, symbol) in text.char_indices() {
        if symbol.is_whitespace() || symbol == '|' || !symbols.contains(symbol) {
            continue;
        }
        if quantityless && !text[..index].trim().is_empty() {
            continue;
        }
        let right = text[index + symbol.len_utf8()..].trim_start();
        let end = right.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let digits = &right[..right.len() - end];
        let Some(d) = digits.parse::<i32>().ok().filter(|d| (1..=100).contains(d)) else {
            continue;
        };
        let q = if quantityless {
            Some(1)
        } else {
            quantity_suffix(&text[..index])
        };
        if let Some(q) = q {
            found = Some((q, d));
        }
    }
    found
}
fn length(text: &str, separate: bool) -> Option<f64> {
    for (index, c) in text.char_indices() {
        if c != 'l' && c != 'L' {
            continue;
        }
        if let Some(previous) = text[..index].chars().last() {
            if (separate && !previous.is_whitespace())
                || (!separate && (previous.is_alphanumeric() || previous == '_'))
            {
                continue;
            }
        }
        let Some(right) = text[index + 1..].trim_start().strip_prefix('=') else {
            continue;
        };
        let right = right.trim_start();
        let count = right.bytes().take_while(|c| c.is_ascii_digit()).count();
        if count == 0 {
            continue;
        }
        let mut end = count;
        if matches!(right.as_bytes().get(end), Some(b'.' | b',')) {
            let decimals = right[end + 1..]
                .bytes()
                .take_while(|c| c.is_ascii_digit())
                .count();
            if decimals > 0 {
                end += 1 + decimals;
            }
        }
        if let Some(value) = right[..end]
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.)
        {
            return Some(value);
        }
    }
    None
}
fn quantity_label(text: &str) -> Option<i32> {
    let upper = text.to_uppercase();
    let index = upper
        .rmatch_indices("ADET")
        .chain(upper.rmatch_indices("ATET"))
        .map(|(i, _)| i)
        .max()?;
    if upper[index + 4..]
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric())
    {
        return None;
    }
    quantity_suffix(&upper[..index])
}

pub struct RebarRead {
    pub rows: Vec<RebarEntry>,
    pub processed: Vec<String>,
    pub unmatched: usize,
}
pub fn read_rebar(
    texts: &[RebarText],
    settings: &RebarSettings,
    deduction: bool,
    drawing: &str,
) -> Result<RebarRead, String> {
    settings.validate()?;
    if texts
        .iter()
        .any(|t| t.position.iter().any(|v| !v.is_finite()))
    {
        return Err("Metin konumu geçersiz.".into());
    }
    let clean: Vec<_> = texts.iter().map(|t| normalize(&t.text)).collect();
    let mut result = RebarRead {
        rows: Vec::new(),
        processed: Vec::new(),
        unmatched: 0,
    };
    let mut add = |q: i32, d: i32, l: f64, indices: &[usize]| -> Result<(), String> {
        let q = q
            .checked_mul(settings.multiplier)
            .ok_or("Adet çarpanı sayısal sınırı aşıyor.")?;
        let mut row = RebarEntry::manual(q, d, l, deduction)?;
        row.source = indices
            .iter()
            .map(|&i| texts[i].text.clone())
            .collect::<Vec<_>>()
            .join(" | ");
        if settings.multiplier != 1 {
            row.source = format!("x{} | {}", settings.multiplier, row.source);
        }
        row.handles = indices
            .iter()
            .map(|&i| texts[i].handle.clone())
            .collect::<Vec<_>>()
            .join("|");
        row.drawing = drawing.into();
        row.layer = texts[indices[0]].layer.clone();
        for &i in indices {
            if !result.processed.contains(&texts[i].handle) {
                result.processed.push(texts[i].handle.clone());
            }
        }
        result.rows.push(row);
        Ok(())
    };
    if settings.mode == RebarMode::Mod1 {
        for (i, text) in clean.iter().enumerate() {
            if let (Some((q, d)), Some(l)) = (
                diameter(text, &settings.symbols, false),
                length(text, false),
            ) {
                add(q, d, l, &[i])?;
            }
        }
    } else if settings.mode == RebarMode::Quantityless {
        let mut caps = Vec::new();
        let mut lengths = Vec::new();
        let mut quantities = Vec::new();
        for (i, text) in clean.iter().enumerate() {
            if let Some((_, d)) = diameter(text, &settings.symbols, true) {
                caps.push((i, d));
            } else if let Some(l) = length(text, true) {
                lengths.push((i, l));
            } else if let Some(q) = quantity_label(text) {
                quantities.push((i, q));
            }
        }
        if caps.len() != 1 || lengths.len() != 1 {
            return Err(format!(
                "Adetsiz: tam bir çap ve bir boy seçin. Bulunan çap: {}, boy: {}.",
                caps.len(),
                lengths.len()
            ));
        }
        let mut q = if quantities.is_empty() { 1i32 } else { 0i32 };
        for &(_, value) in &quantities {
            q = q.checked_add(value).ok_or("Adet toplamı çok büyük.")?;
        }
        let mut indices = vec![caps[0].0, lengths[0].0];
        indices.extend(quantities.iter().map(|(i, _)| *i));
        add(q, caps[0].1, lengths[0].1, &indices)?;
    } else {
        let mut caps = Vec::new();
        let mut lengths = Vec::new();
        for (i, text) in clean.iter().enumerate() {
            if let Some((q, d)) = diameter(text, &settings.symbols, false) {
                caps.push((i, q, d));
            } else if let Some(l) = length(text, true) {
                lengths.push((i, l));
            }
        }
        let mut used = std::collections::HashSet::new();
        let mut unmatched = 0;
        for (i, q, d) in caps {
            let mut best: Option<(usize, usize, f64, f64)> = None;
            for (slot, &(j, l)) in lengths.iter().enumerate() {
                if used.contains(&slot) {
                    continue;
                }
                let delta: Vec<_> = texts[j]
                    .position
                    .iter()
                    .zip(texts[i].position)
                    .map(|(a, b)| a - b)
                    .collect();
                let score = match settings.mode {
                    RebarMode::Mod2 => delta.iter().map(|v| v * v).sum::<f64>(),
                    RebarMode::Horizontal
                        if delta[1].abs() <= settings.tolerance && delta[0] > 0.0001 =>
                    {
                        delta[0]
                    }
                    RebarMode::Vertical
                        if delta[0].abs() <= settings.tolerance && delta[1] > 0.0001 =>
                    {
                        delta[1]
                    }
                    _ => continue,
                };
                if best.as_ref().is_none_or(|(_, _, _, old)| score < *old) {
                    best = Some((slot, j, l, score));
                }
            }
            if let Some((slot, j, l, _)) = best {
                used.insert(slot);
                add(q, d, l, &[i, j])?;
            } else {
                unmatched += 1;
            }
        }
        result.unmatched = unmatched;
    }
    if result.rows.is_empty() {
        return Err(format!(
            "Geçerli demir tarifi bulunamadı. {} eşleşmeyen adet/çap yazısı.",
            result.unmatched
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn texts(values: &[(&str, [f64; 3])]) -> Vec<RebarText> {
        values
            .iter()
            .enumerate()
            .map(|(i, (text, position))| RebarText {
                handle: i.to_string(),
                text: (*text).into(),
                position: *position,
                layer: "DONATI".into(),
            })
            .collect()
    }
    #[test]
    fn parses_reference_examples_and_rejects_invalid_products() {
        for (source, q, d, l) in [
            ("TK 20 52Φ10 L=364", 52, 10, 364.),
            ("3x2ø8 L=100", 6, 8, 100.),
            ("3 × 2 Φ8 L=100", 6, 8, 100.),
            ("3*2Ø8 L=100", 6, 8, 100.),
            ("TK 20 3x2x4ø8 L=100", 24, 8, 100.),
            ("12Ø16 L=250,5", 12, 16, 250.5),
            ("12%%c16\\PL=250", 12, 16, 250.),
            ("12∅16 L=250", 12, 16, 250.),
        ] {
            let read = read_rebar(
                &texts(&[(source, [0.; 3])]),
                &RebarSettings::default(),
                false,
                "plan",
            )
            .unwrap();
            let row = &read.rows[0];
            assert_eq!(
                (row.quantity, row.diameter_mm, row.length_cm),
                (q, d, l),
                "{source}"
            );
        }
        for bad in [
            "0x2ø8",
            "2147483647x2ø8",
            "3xx2ø8",
            "3x0ø8",
            "-2Ø8",
            "1.5Ø8",
            "2Ø101",
        ] {
            assert!(
                read_rebar(
                    &texts(&[(&format!("{bad} L=100"), [0.; 3])]),
                    &RebarSettings::default(),
                    false,
                    "plan"
                )
                .is_err(),
                "{bad}"
            );
        }
        let row = RebarEntry::manual(12, 16, 250., false).unwrap();
        assert_eq!(row.total_length_m(), 30.);
        assert!((row.weight_kg() - 47.4074074074074).abs() < 1e-10);
    }
    #[test]
    fn pairing_uses_each_length_once_and_obeys_direction_and_tolerance() {
        for mode in [RebarMode::Mod2, RebarMode::Horizontal, RebarMode::Vertical] {
            let positions = if mode == RebarMode::Vertical {
                [[0., 0., 0.], [0., 10., 0.], [0.005, 3., 0.]]
            } else {
                [[0., 0., 0.], [10., 0., 0.], [3., 0.005, 0.]]
            };
            let input = texts(&[
                ("12Ø16", positions[0]),
                ("8Ø12", positions[1]),
                ("L=250", positions[2]),
            ]);
            let settings = RebarSettings {
                mode,
                ..Default::default()
            };
            let read = read_rebar(&input, &settings, false, "plan").unwrap();
            assert_eq!(read.rows.len(), 1);
            assert_eq!(read.unmatched, 1);
            assert_eq!(read.processed.len(), 2);
            assert_eq!(read.rows[0].quantity, 12);
            if mode != RebarMode::Mod2 {
                let strict = RebarSettings {
                    tolerance: 0.001,
                    ..settings
                };
                assert!(read_rebar(&input, &strict, false, "plan").is_err());
            }
        }
        let left = texts(&[("12Ø16", [0.; 3]), ("L=250", [-3., 0., 0.])]);
        assert!(read_rebar(
            &left,
            &RebarSettings {
                mode: RebarMode::Horizontal,
                ..Default::default()
            },
            false,
            "p"
        )
        .is_err());
        assert!(read_rebar(
            &left,
            &RebarSettings {
                mode: RebarMode::Mod2,
                ..Default::default()
            },
            false,
            "p"
        )
        .is_ok());
    }
    #[test]
    fn quantityless_combines_quantity_labels_and_supports_default_and_deductions() {
        let input = texts(&[
            ("Ø8/20/10", [0.; 3]),
            ("L=100", [2.; 3]),
            ("12 Adet", [9.; 3]),
            ("3x2 Adet", [20.; 3]),
        ]);
        let settings = RebarSettings {
            mode: RebarMode::Quantityless,
            multiplier: 2,
            ..Default::default()
        };
        let read = read_rebar(&input, &settings, true, "plan").unwrap();
        assert_eq!(read.rows[0].quantity, 36);
        assert_eq!(read.rows[0].total_length_m(), -36.);
        assert_eq!(read.processed.len(), 4);
        let default = read_rebar(&input[..2], &settings, false, "plan").unwrap();
        assert_eq!(default.rows[0].quantity, 2);
        let duplicate = texts(&[("Ø8", [0.; 3]), ("Ø10", [0.; 3]), ("L=100", [0.; 3])]);
        assert!(read_rebar(&duplicate, &settings, false, "plan").is_err());
        let mut huge = settings.clone();
        huge.multiplier = i32::MAX;
        assert!(read_rebar(&input, &huge, false, "plan").is_err());
        assert!(RebarEntry::manual(1, 16, f64::INFINITY, false).is_err());
    }
}
