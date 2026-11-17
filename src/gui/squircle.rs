//! Squircle geometry: superellipse corners for egui surfaces.
//!
//! A squircle (superellipse) corner reads fuller and more premium than a
//! circular arc of the same radius. This module renders rounded rectangles as
//! a single convex polygon using epaint's feathered path tessellation, so the
//! anti-aliasing is identical to egui's built-in rounded rects.
//!
//! The unit corner arc is computed ONCE (cached in a `OnceLock`) with a fixed
//! superellipse exponent and segment count. Painting only scales/offsets that
//! template, so there is no per-frame `powf`/`sin` cost and no hardware spike.
//!
//! All content is pure ASCII.

use eframe::egui::{Color32, Painter, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use std::sync::OnceLock;

/// Superellipse exponent. 4.0 closely matches the iOS squircle; higher values
/// approach a rectangle, 2.0 is a plain circle.
pub const SUPERELLIPSE_N: f32 = 4.0;

/// Segments per corner arc. 12 is visually smooth at every UI scale.
const SEGMENTS: usize = 12;

/// Unit superellipse quarter-arc, normalized to [0,1] on both axes.
/// Index 0 is (1, 0) and the last index is (0, 1).
fn unit_corner() -> &'static Vec<Vec2> {
    static TEMPLATE: OnceLock<Vec<Vec2>> = OnceLock::new();
    TEMPLATE.get_or_init(|| {
        let exponent = 2.0 / SUPERELLIPSE_N;
        (0..=SEGMENTS)
            .map(|i| {
                let t = (i as f32 / SEGMENTS as f32) * std::f32::consts::FRAC_PI_2;
                // cos(pi/2) can be a tiny NEGATIVE float; powf on a negative
                // base with a fractional exponent returns NaN, which poisons
                // the whole polygon. Clamp to [0,1] before the power.
                let cx = t.cos().clamp(0.0, 1.0);
                let sy = t.sin().clamp(0.0, 1.0);
                Vec2::new(cx.powf(exponent), sy.powf(exponent))
            })
            .collect()
    })
}

/// Clamp a corner radius to the rect's half-extents (egui does the same for
/// `Rounding`), so degenerate radii never fold the polygon inward.
fn clamp_radius(r: f32, rect: Rect) -> f32 {
    r.max(0.0).min(rect.width() * 0.5).min(rect.height() * 0.5)
}

/// Build the squircle perimeter as a clockwise convex polygon.
///
/// The four superellipse corner arcs are concatenated; straight rect edges
/// are implicit between consecutive corner endpoints. Winding is forced
/// clockwise (negative signed area in screen coordinates) for epaint's
/// preferred fill path.
pub fn squircle_path(rect: Rect, rounding: Rounding) -> Vec<Pos2> {
    let template = unit_corner();
    let nw = clamp_radius(rounding.nw, rect);
    let ne = clamp_radius(rounding.ne, rect);
    let sw = clamp_radius(rounding.sw, rect);
    let se = clamp_radius(rounding.se, rect);

    let mut points = Vec::with_capacity(4 * (SEGMENTS + 1));

    // (corner center x, center y, x sign, y sign, radius)
    let corners = [
        (rect.max.x - ne, rect.min.y + ne, 1.0, -1.0, ne), // top-right
        (rect.max.x - se, rect.max.y - se, 1.0, 1.0, se),  // bottom-right
        (rect.min.x + sw, rect.max.y - sw, -1.0, 1.0, sw), // bottom-left
        (rect.min.x + nw, rect.min.y + nw, -1.0, -1.0, nw), // top-left
    ];

    for (cx, cy, sx, sy, r) in corners {
        for u in template {
            points.push(Pos2::new(cx + sx * u.x * r, cy + sy * u.y * r));
        }
    }

    // Force clockwise winding for epaint's convex fill path.
    let twice_area: f32 = points
        .windows(2)
        .map(|pair| pair[0].x * pair[1].y - pair[1].x * pair[0].y)
        .sum();
    if twice_area > 0.0 {
        points.reverse();
    }

    points
}

/// Paint a filled squircle (superellipse-cornered rect).
pub fn squircle_filled(painter: &Painter, rect: Rect, rounding: Rounding, fill: Color32) {
    painter.add(Shape::convex_polygon(
        squircle_path(rect, rounding),
        fill,
        Stroke::NONE,
    ));
}

/// Paint a squircle outline with no fill.
pub fn squircle_stroke(painter: &Painter, rect: Rect, rounding: Rounding, stroke: Stroke) {
    painter.add(Shape::convex_polygon(
        squircle_path(rect, rounding),
        Color32::TRANSPARENT,
        stroke,
    ));
}

/// Paint a squircle with both fill and stroke in a single tessellation.
pub fn squircle(painter: &Painter, rect: Rect, rounding: Rounding, fill: Color32, stroke: Stroke) {
    painter.add(Shape::convex_polygon(
        squircle_path(rect, rounding),
        fill,
        stroke,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_is_unit_quarter_arc() {
        let arc = unit_corner();
        assert_eq!(arc.len(), SEGMENTS + 1);
        // Endpoint (1,0) and (0,1).
        assert!((arc[0].x - 1.0).abs() < 1e-5 && arc[0].y.abs() < 1e-5);
        let last = arc.last().unwrap();
        assert!(last.x.abs() < 1e-5 && (last.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn superellipse_is_fuller_than_circle() {
        let arc = unit_corner();
        let mid = arc[SEGMENTS / 2];
        // A circular quarter would be ~0.707 at 45 degrees; n=4 is ~0.84.
        assert!(mid.x > 0.8 && mid.y > 0.8, "mid={:?}", mid);
    }

    #[test]
    fn path_is_clockwise_and_bounded() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(40.0));
        let path = squircle_path(rect, Rounding::same(10.0));
        assert_eq!(path.len(), 4 * (SEGMENTS + 1));
        let twice_area: f32 = path
            .windows(2)
            .map(|p| p[0].x * p[1].y - p[1].x * p[0].y)
            .sum();
        assert!(twice_area < 0.0, "expected clockwise, area={}", twice_area);
        for p in &path {
            assert!(p.x >= -0.001 && p.x <= 40.001 && p.y >= -0.001 && p.y <= 40.001);
        }
    }
}
