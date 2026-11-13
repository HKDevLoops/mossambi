//! Icon snap arithmetic tests: snapped_icon_quad must produce a
//! square whose device size equals the icon_texture rasterization
//! and whose min corner sits on integer device pixels.

use eframe::egui::{Context, Pos2, RawInput, Rect, Vec2};
use mossambi::core::icon_packs::{
    mix_hex, shade_hex, snapped_icon_quad, svg_for, IconGlyph, IconPack,
};

/// Fresh headless egui context with pixels-per-point APPLIED.
/// set_pixels_per_point only takes effect at the next begin_pass
/// (verified empirically against egui 0.29), so a pass is begun
/// here, exactly like a real frame.
fn ctx_at(ppp: f32) -> Context {
    let ctx = Context::default();
    ctx.set_pixels_per_point(ppp);
    ctx.begin_pass(RawInput::default());
    ctx
}

/// Builds a fractional-centered square rect (like a table row) and
/// returns the snapped paint quad for css_px at the context's ppp.
fn quad(ctx: &Context, center: (f32, f32), css_px: f32) -> Rect {
    let rect = Rect::from_center_size(Pos2::new(center.0, center.1), Vec2::splat(css_px + 8.0));
    snapped_icon_quad(ctx, rect, css_px)
}

#[test]
fn test_quad_ppp_1_snaps_to_integers() {
    let ctx = ctx_at(1.0);
    assert_eq!(ctx.pixels_per_point(), 1.0);
    let q = quad(&ctx, (10.5, 20.25), 20.0);
    assert_eq!(q.width(), 20.0);
    assert_eq!(q.height(), 20.0);
    assert_eq!(q.min.x.fract(), 0.0);
    assert_eq!(q.min.y.fract(), 0.0);
}

#[test]
fn test_quad_ppp_125_and_150_match_texture() {
    for ppp in [1.25f32, 1.5f32] {
        let ctx = ctx_at(ppp);
        assert_eq!(
            ctx.pixels_per_point(),
            ppp,
            "the requested ppp must be ACTIVE, not deferred"
        );
        let q = quad(&ctx, (10.5, 20.25), 20.0);
        let phys_w = q.width() * ppp;
        assert!(
            (phys_w - phys_w.round()).abs() < 1e-3,
            "device width must be integral at ppp {ppp}"
        );
        assert_eq!(
            phys_w.round() as u32,
            ((20.0 * ppp).round() as u32).clamp(12, 512),
            "device width must equal the icon_texture phys size at ppp {ppp}"
        );
        let dx = q.min.x * ppp;
        let dy = q.min.y * ppp;
        assert!(
            (dx - dx.round()).abs() < 1e-3 && (dy - dy.round()).abs() < 1e-3,
            "min corner must sit on integer device pixels at ppp {ppp}"
        );
    }
}

#[test]
fn test_quad_small_request_clamps_to_12_device_px() {
    let ctx = ctx_at(1.0);
    let q = quad(&ctx, (5.5, 5.5), 2.0);
    assert_eq!(
        q.width(),
        12.0,
        "2 px request must clamp to the 12 device-pixel floor"
    );
}

#[test]
fn test_quad_huge_request_clamps_to_512_device_px() {
    let ctx = ctx_at(1.0);
    let q = quad(&ctx, (100.0, 100.0), 6000.0);
    assert_eq!(
        q.width(),
        512.0,
        "oversize request must clamp to 512 device pixels"
    );
}

// -- Icon wave: hex helpers and glassy folder anatomy ------------------------

#[test]
fn shade_hex_scales_channels() {
    assert_eq!(shade_hex("#FFFFFF", 0.5), "#808080");
    assert_eq!(shade_hex("#FFFFFF", 1.0), "#FFFFFF");
    assert_eq!(shade_hex("#FFFFFF", 0.0), "#000000");
    assert_eq!(shade_hex("not-a-hex", 0.5), "not-a-hex");
}

#[test]
fn mix_hex_blends_channels() {
    assert_eq!(mix_hex("#FF0000", "#FFFFFF", 0.5), "#FF8080");
    assert_eq!(mix_hex("#FF0000", "#FFFFFF", 0.0), "#FF0000");
    assert_eq!(mix_hex("#FF0000", "#FFFFFF", 1.0), "#FFFFFF");
    assert_eq!(mix_hex("bad", "#FFFFFF", 0.5), "bad");
}

#[test]
fn finder_folder_svg_has_glass_anatomy() {
    let svg = svg_for(IconPack::MacosFinder, IconGlyph::Folder, [75, 75, 75]);
    assert!(svg.contains("<linearGradient"), "gradient element missing");
    assert!(svg.contains("ggfront"), "gradient id missing");
    assert!(svg.contains("url(#ggfront)"), "front glass fill missing");
    assert!(svg.contains("opacity='0.35'"), "specular top edge missing");
    assert!(svg.contains("opacity='0.14'"), "bottom catch missing");
}
