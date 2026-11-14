//! Icon snap arithmetic tests: snapped_icon_quad must produce a
//! square whose device size equals the icon_texture rasterization
//! and whose min corner sits on integer device pixels.

use eframe::egui::{Context, Pos2, RawInput, Rect, Vec2};
use mossambi::core::icon_packs::{
    mix_hex, shade_hex, snapped_icon_quad, special_folder_svg, special_folder_svg_sized, svg_for,
    IconGlyph, IconPack,
};
use mossambi::core::SpecialFolder;

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

#[test]
fn test_every_pack_x_every_kind_parses_and_differs_from_generic() {
    let opt = resvg::usvg::Options::default();
    let packs = IconPack::all();
    let kinds = SpecialFolder::all();

    for pack in packs {
        let generic_svg = svg_for(pack, IconGlyph::Folder, [0, 0, 0]);

        for kind in kinds {
            // 1. Regular size SVG
            let svg = special_folder_svg(pack, *kind);
            assert_eq!(
                svg.matches("</svg>").count(),
                1,
                "SVG must contain exactly one </svg> for {pack:?} {kind:?}"
            );
            let tree = resvg::usvg::Tree::from_str(&svg, &opt);
            assert!(
                tree.is_ok(),
                "Failed to parse regular SVG for {pack:?} {kind:?}: {:?}",
                tree.err()
            );
            assert_ne!(
                svg, generic_svg,
                "Special folder {kind:?} must differ from generic folder in {pack:?}"
            );

            // 2. Small size (<= 20px) simplified thicker stroke SVG
            let small_svg = special_folder_svg_sized(pack, *kind, true);
            assert_eq!(
                small_svg.matches("</svg>").count(),
                1,
                "Small SVG must contain exactly one </svg> for {pack:?} {kind:?}"
            );
            let small_tree = resvg::usvg::Tree::from_str(&small_svg, &opt);
            assert!(
                small_tree.is_ok(),
                "Failed to parse small SVG for {pack:?} {kind:?}: {:?}",
                small_tree.err()
            );
        }
    }
}

#[test]
fn test_known_folder_path_lookup() {
    let desktop = mossambi::core::platform_paths::desktop_dir();
    assert_eq!(
        mossambi::core::special_folders::lookup_path(&desktop),
        Some(SpecialFolder::Desktop)
    );

    let downloads = mossambi::core::platform_paths::downloads_dir();
    assert_eq!(
        mossambi::core::special_folders::lookup_path(&downloads),
        Some(SpecialFolder::Downloads)
    );

    let documents = mossambi::core::platform_paths::documents_dir();
    assert_eq!(
        mossambi::core::special_folders::lookup_path(&documents),
        Some(SpecialFolder::Documents)
    );
}

#[test]
fn test_folder_name_fallback() {
    let cases = [
        ("Desktop", SpecialFolder::Desktop),
        ("Downloads", SpecialFolder::Downloads),
        ("My Documents", SpecialFolder::Documents),
        ("Pictures", SpecialFolder::Pictures),
        ("Videos", SpecialFolder::Videos),
        ("Music", SpecialFolder::Music),
        ("OneDrive", SpecialFolder::OneDrive),
        ("$Recycle.Bin", SpecialFolder::RecycleBin),
        ("Favorites", SpecialFolder::Favorites),
        ("Saved Games", SpecialFolder::SavedGames),
        ("Links", SpecialFolder::Links),
        ("Contacts", SpecialFolder::Contacts),
        ("Searches", SpecialFolder::Searches),
        ("3D Objects", SpecialFolder::Objects3D),
        ("Program Files", SpecialFolder::ProgramFiles),
        ("Users", SpecialFolder::Users),
    ];

    for (name, expected) in cases {
        let resolved = mossambi::core::special_folders::lookup_name(name);
        assert_eq!(
            resolved,
            Some(expected),
            "Name fallback failed for '{name}'"
        );

        let glyph = IconGlyph::folder_special(name);
        assert_eq!(
            glyph,
            Some(IconGlyph::FolderSpecial(expected)),
            "folder_special glyph failed for '{name}'"
        );
    }
}
