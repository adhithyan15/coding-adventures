//! # VER08b runtime capability conformance for `swf-shape-to-paint`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use std::path::PathBuf;

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use swf_container::SwfRect;
use swf_shape_to_paint::{
    checked_twips_add, checked_twips_point_add, evaluate_quad_edge, flatten_quad_edge,
    lower_shape_to_paint_scene, pixels_to_twips_i32, quad_edge_bounding_box, reconstruct_all_fills,
    reconstruct_all_strokes, reconstruct_fill_contours, reconstruct_stroke_runs,
    twips_point_to_pixels, twips_to_pixels_f64, validate_shape_tag, validate_style_count,
    validate_twips, ColorRgba, ParsedShape, ShapeEdge, SwfCapStyle, SwfFillStyle, SwfJoinStyle,
    SwfLineStyle,
};

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        // 1. Validation functions
        checks.push(check("validate_shape_tag", validate_shape_tag(2).is_ok()));
        checks.push(check("validate_twips", validate_twips(100).is_ok()));
        checks.push(check(
            "validate_style_count",
            validate_style_count(10).is_ok(),
        ));

        // 2. Twips conversions
        let px = twips_to_pixels_f64(40);
        let tw = pixels_to_twips_i32(px);
        let pt = twips_point_to_pixels(tw, tw);
        checks.push(check(
            "twips roundtrip",
            px == 2.0 && tw == 40 && pt == (2.0, 2.0),
        ));

        let add_res = checked_twips_add(10, 20);
        let pt_res = checked_twips_point_add(10, 20, 5, 5);
        checks.push(check(
            "checked twips math",
            add_res == Ok(30) && pt_res == Ok((15, 25)),
        ));

        // 3. Styles and color
        let color = ColorRgba::rgba(255, 128, 64, 255);
        checks.push(check(
            "color to css",
            color.color_to_css_string() == "#ff8040",
        ));

        let cap = SwfCapStyle::Round.cap_to_paint_cap();
        let join = SwfJoinStyle::Miter.join_to_paint_join();
        checks.push(check(
            "cap and join",
            cap == paint_instructions::StrokeCap::Round
                && join == paint_instructions::StrokeJoin::Miter,
        ));

        // 4. Bezier2d integration
        let edge = ShapeEdge {
            start: (0, 0),
            end: (200, 0),
            control: Some((100, 200)),
            fill_style0: 1,
            fill_style1: 0,
            line_style: 1,
        };
        let p_mid = evaluate_quad_edge(&edge, 0.5);
        let bbox = quad_edge_bounding_box(&edge);
        let poly = flatten_quad_edge(&edge, 0.5);
        checks.push(check(
            "bezier evaluation",
            p_mid == (5.0, 5.0) && bbox.2 == 10.0 && poly.is_ok(),
        ));

        // 5. Contours and stroke reconstruction
        let edges = vec![
            ShapeEdge {
                start: (0, 0),
                end: (100, 0),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 1,
            },
            ShapeEdge {
                start: (100, 0),
                end: (100, 100),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 1,
            },
            ShapeEdge {
                start: (100, 100),
                end: (0, 0),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 1,
            },
        ];

        let contours = reconstruct_fill_contours(&edges, 1);
        let stroke_runs = reconstruct_stroke_runs(&edges, 1);
        checks.push(check(
            "reconstruction",
            contours.len() == 1 && contours[0].is_closed && stroke_runs.len() == 1,
        ));

        // 6. ParsedShape lowering
        let parsed = ParsedShape {
            tag_code: 2,
            shape_id: 1,
            bounds: SwfRect::create_rect(0, 100, 0, 100),
            edge_bounds: None,
            uses_non_zero_winding: false,
            fill_styles: vec![SwfFillStyle::Solid(color)],
            line_styles: vec![SwfLineStyle {
                width_twips: 20,
                color,
                cap_style: SwfCapStyle::Round,
                join_style: SwfJoinStyle::Round,
                miter_limit: 3.0,
                no_close: false,
                fill_style: None,
            }],
            edges,
        };

        let all_fills = reconstruct_all_fills(&parsed);
        let all_strokes = reconstruct_all_strokes(&parsed);
        let scene = lower_shape_to_paint_scene(&parsed);
        checks.push(check(
            "lowering to scene",
            all_fills.len() == 1 && all_strokes.len() == 1 && scene.instructions.len() == 2,
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/swf-shape-to-paint",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
