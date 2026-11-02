use crate::entities::traits::EntityTypeOps;
use crate::scene::{convert::acad_to_render::RenderObject, Scene};
use codec::{EntityType, Handle};
use glam::{DVec2, DVec3};

pub(super) fn match_colors(
    mode: crate::metraj::rebar::RebarMarking,
    single: u8,
    count: usize,
    offset: usize,
) -> Vec<u8> {
    if mode == crate::metraj::rebar::RebarMarking::Color {
        return vec![single; count];
    }
    static PALETTE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let palette = PALETTE.get_or_init(|| {
        let mut used = std::collections::HashSet::new();
        (1..=255u8)
            .filter(|&index| {
                codec::types::aci_table::aci_to_rgb(index).is_some_and(|rgb| used.insert(rgb))
            })
            .collect()
    });
    (0..count)
        .map(|i| palette[(offset + i) % palette.len()])
        .collect()
}

/// Use the renderer's ink, not the insertion point or the MTEXT editing width.
fn ink_points(scene: &Scene, handle: Handle) -> Result<(Vec<DVec3>, DVec3), String> {
    let entity = scene
        .document
        .get_entity(handle)
        .ok_or("Eşleşen yazı bulunamadı.")?;
    let anno = crate::scene::annotative::effective_annotation_scale_for(
        &scene.document,
        entity,
        scene.annotation_scale,
        scene.displayed_annotation_scale_handle(),
    );
    let render = match entity {
        EntityType::Text(t) => Some(crate::entities::text::to_render_at_scale(
            t,
            &scene.document,
            anno,
        )),
        EntityType::MText(_) => entity.to_render_entity(&scene.document),
        _ => None,
    }
    .ok_or("Bounding box için TEXT/MTEXT gerekir.")?;
    let RenderObject::Text(groups) = render.object else {
        return Err("Yazı geometrisi okunamadı.".into());
    };
    let (reference, z) = match entity {
        EntityType::MText(t) => (
            [t.insertion_point.x, t.insertion_point.y],
            t.insertion_point.z,
        ),
        EntityType::Text(t) => (
            groups.first().map(|g| g.origin).unwrap_or([0.; 2]),
            t.insertion_point.z,
        ),
        _ => unreachable!(),
    };
    let mut points = Vec::new();
    let mut normal = DVec3::Z;
    let scale = anno as f64;
    for group in groups {
        let axis_x = group.plane.map(|p| DVec3::from(p.x_axis));
        let axis_y = group.plane.map(|p| DVec3::from(p.y_axis));
        for p in group.strokes.iter().flatten().chain(group.fill_tris.iter()) {
            if p.iter().any(|v| !v.is_finite()) {
                continue;
            }
            let point = if let Some(plane) = group.plane {
                let origin = DVec3::from(plane.scale_origin)
                    + (DVec3::from(plane.origin) - DVec3::from(plane.scale_origin)) * scale;
                origin
                    + axis_x.unwrap() * p[0] as f64 * scale
                    + axis_y.unwrap() * p[1] as f64 * scale
            } else {
                DVec3::new(
                    reference[0] + (group.origin[0] - reference[0] + p[0] as f64) * scale,
                    reference[1] + (group.origin[1] - reference[1] + p[1] as f64) * scale,
                    z,
                )
            };
            if !point.is_finite() {
                return Err("Yazı sınırı sonlu değil.".into());
            }
            if points.is_empty() {
                normal = axis_x
                    .zip(axis_y)
                    .map(|(x, y)| x.cross(y).normalize())
                    .unwrap_or(DVec3::Z);
            }
            points.push(point);
        }
    }
    if points.len() < 3 {
        return Err("Görünür yazı sınırı hesaplanamadı; kutu oluşturulmadı.".into());
    }
    Ok((points, normal))
}

fn cross(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    (b - a).perp_dot(c - a)
}

/// A minimum-area enclosing rectangle has an edge parallel to a hull edge.
fn minimum_rectangle(points: &[DVec2]) -> Result<[DVec2; 4], String> {
    let mut sorted = points.to_vec();
    if sorted.iter().any(|p| !p.is_finite()) {
        return Err("Kutu noktası geçersiz.".into());
    }
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    sorted.dedup();
    let mut lower: Vec<DVec2> = Vec::new();
    for &p in &sorted {
        while lower.len() > 1 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0. {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<DVec2> = Vec::new();
    for &p in sorted.iter().rev() {
        while upper.len() > 1 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0. {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    let hull = lower;
    if hull.len() < 3 {
        return Err("Yazı grubu için kapalı kutu hesaplanamadı.".into());
    }
    let origin = hull[0];
    let mut best: Option<(f64, [DVec2; 4])> = None;
    for i in 0..hull.len() {
        let u = (hull[(i + 1) % hull.len()] - hull[i]).normalize();
        let v = DVec2::new(-u.y, u.x);
        let mut lo = DVec2::splat(f64::INFINITY);
        let mut hi = DVec2::splat(f64::NEG_INFINITY);
        for p in &hull {
            let p = *p - origin;
            let projected = DVec2::new(p.dot(u), p.dot(v));
            lo = lo.min(projected);
            hi = hi.max(projected);
        }
        let size = hi - lo;
        let area = size.x * size.y;
        if !area.is_finite() || area <= 0. {
            continue;
        }
        if best.as_ref().is_none_or(|(old, _)| area < *old) {
            best = Some((
                area,
                [
                    origin + u * lo.x + v * lo.y,
                    origin + u * hi.x + v * lo.y,
                    origin + u * hi.x + v * hi.y,
                    origin + u * lo.x + v * hi.y,
                ],
            ));
        }
    }
    best.map(|(_, corners)| corners)
        .ok_or_else(|| "En küçük yazı kutusu hesaplanamadı.".into())
}

pub(super) fn enclosing_box(scene: &Scene, handles: &str, color: u8) -> Result<EntityType, String> {
    let mut points = Vec::new();
    let mut normal = DVec3::Z;
    for hex in handles.split('|') {
        let handle = Handle::new(u64::from_str_radix(hex, 16).map_err(|e| e.to_string())?);
        let (ink, n) = ink_points(scene, handle)?;
        if points.is_empty() {
            normal = n;
        }
        points.extend(ink);
    }
    let anchor = *points.first().ok_or("Kutu için eşleşen yazı yok.")?;
    let span = points
        .iter()
        .map(|p| p.distance(anchor))
        .fold(1.0, f64::max);
    if points
        .iter()
        .any(|p| (*p - anchor).dot(normal).abs() > span * 1e-7)
    {
        return Err("Aynı kaydın yazıları ortak düzlemde değil; bounding box için aynı düzlemde yazılar seçin.".into());
    }
    let n = codec::types::Vector3::new(normal.x, normal.y, normal.z);
    let basis = codec::types::Matrix3::arbitrary_axis(n);
    let x = basis * codec::types::Vector3::new(1., 0., 0.);
    let y = basis * codec::types::Vector3::new(0., 1., 0.);
    let x = DVec3::new(x.x, x.y, x.z);
    let y = DVec3::new(y.x, y.y, y.z);
    let local: Vec<_> = points
        .iter()
        .map(|p| DVec2::new((*p - anchor).dot(x), (*p - anchor).dot(y)))
        .collect();
    let corners = minimum_rectangle(&local)?;
    let mut polyline = codec::entities::LwPolyline::new();
    polyline.is_closed = true;
    polyline.normal = n;
    polyline.elevation = anchor.dot(normal);
    polyline.common.layer = "HESAPLANDI".into();
    polyline.common.color = codec::types::Color::Index(color);
    polyline.vertices = corners
        .iter()
        .map(|p| codec::entities::LwVertex::from_coords(anchor.dot(x) + p.x, anchor.dot(y) + p.y))
        .collect();
    Ok(EntityType::LwPolyline(polyline))
}

/// Replace only real inline colour controls, leaving fonts and layout intact.
fn recolor_inline(value: &str, color: u8) -> String {
    let mut output = String::with_capacity(value.len());
    let mut index = 0;
    while index < value.len() {
        let rest = &value[index..];
        if rest.starts_with("\\\\") {
            output.push_str("\\\\");
            index += 2;
            continue;
        }
        if rest.starts_with("\\C") || rest.starts_with("\\c") {
            let digits = rest[2..].bytes().take_while(|c| c.is_ascii_digit()).count();
            if digits > 0 && rest.as_bytes().get(2 + digits) == Some(&b';') {
                output.push_str(&format!("\\C{color};"));
                index += 3 + digits;
                continue;
            }
        }
        if rest.starts_with('\\') && rest.chars().nth(1).is_some_and(|c| "fFHTWQApS".contains(c)) {
            if let Some(end) = rest.find(';') {
                output.push_str(&rest[..=end]);
                index += end + 1;
                continue;
            }
        }
        let ch = rest.chars().next().unwrap();
        output.push(ch);
        index += ch.len_utf8();
    }
    output
}
pub(super) fn recolor(entity: &EntityType, color: u8) -> EntityType {
    let mut entity = entity.clone();
    entity.common_mut().layer = "HESAPLANDI".into();
    entity.common_mut().color = codec::types::Color::Index(color);
    if let EntityType::MText(t) = &mut entity {
        t.value = recolor_inline(&t.value, color);
    }
    entity
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_palette_starts_red_yellow_and_uses_distinct_rgb_values() {
        let colors = match_colors(crate::metraj::rebar::RebarMarking::Mixed, 6, 16, 0);
        assert_eq!(&colors[..2], &[1, 2]);
        let rgb: std::collections::HashSet<_> = colors
            .iter()
            .map(|&c| codec::types::aci_table::aci_to_rgb(c).unwrap())
            .collect();
        assert_eq!(rgb.len(), 16);
        assert_eq!(
            match_colors(crate::metraj::rebar::RebarMarking::Color, 6, 3, 0),
            vec![6, 6, 6]
        );
        assert_eq!(
            match_colors(crate::metraj::rebar::RebarMarking::Mixed, 6, 2, 2),
            vec![3, 4]
        );
    }
    fn area(corners: [DVec2; 4]) -> f64 {
        (corners[1] - corners[0]).length() * (corners[3] - corners[0]).length()
    }
    #[test]
    fn minimum_box_is_tight_rotated_and_stable_far_from_origin() {
        let angle = 0.63f64;
        let u = DVec2::new(angle.cos(), angle.sin());
        let v = DVec2::new(-u.y, u.x);
        let offset = DVec2::new(1e9, -1e9);
        let points = [
            offset,
            offset + u * 8.,
            offset + u * 8. + v * 2.,
            offset + v * 2.,
            offset + u * 3. + v,
        ];
        let corners = minimum_rectangle(&points).unwrap();
        assert!((area(corners) - 16.).abs() < 1e-5);
        let a = corners[1] - corners[0];
        let b = corners[3] - corners[0];
        for p in points {
            let d = p - corners[0];
            assert!(d.dot(a) >= -1e-5 && d.dot(a) <= a.length_squared() + 1e-5);
            assert!(d.dot(b) >= -1e-5 && d.dot(b) <= b.length_squared() + 1e-5);
        }
        assert!(minimum_rectangle(&[DVec2::ZERO, DVec2::X]).is_err());
    }
    #[test]
    fn inline_color_override_preserves_layout_and_escaped_controls() {
        let before = r"{\fArial|b1;\C1;12Ø16}\P\c16711680;L=250 \\C2;";
        let after = recolor_inline(before, 6);
        assert_eq!(after, r"{\fArial|b1;\C6;12Ø16}\P\C6;L=250 \\C2;");
        assert_eq!(
            codec::entities::mtext_format::parse_mtext(before, true).to_plain_text(),
            codec::entities::mtext_format::parse_mtext(&after, true).to_plain_text()
        );
        assert_eq!(recolor_inline(r"\fC:\C1;A\C2;B", 6), r"\fC:\C1;A\C6;B");
    }
}
