//! # VER08b runtime capability conformance for `flash-display-list`.
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
use flash_display_list::{
    diff_paint_scene_into, parse_place_object2, parse_remove_object2, validate_display_depth,
    validate_hierarchy_depth, validate_morph_ratio, ButtonEvent, ButtonState, DisplayListEvent,
    DisplayObject, DisplaySlot, FrameTarget, MorphShape, MovieClip, PatchOp, PlayState,
    SimpleButton, Stage, TextField,
};
use swf_container::SwfRect;
use swf_shape_to_paint::{ColorRgba, ParsedShape, ShapeEdge, SwfFillStyle};

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn create_sample_shape() -> ParsedShape {
    ParsedShape {
        tag_code: 2,
        shape_id: 1,
        bounds: SwfRect::create_rect(0, 200, 0, 200),
        edge_bounds: None,
        uses_non_zero_winding: false,
        fill_styles: vec![SwfFillStyle::Solid(ColorRgba::rgba(255, 0, 0, 255))],
        line_styles: Vec::new(),
        edges: vec![
            ShapeEdge {
                start: (0, 0),
                end: (200, 0),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 0,
            },
            ShapeEdge {
                start: (200, 0),
                end: (200, 200),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 0,
            },
            ShapeEdge {
                start: (200, 200),
                end: (0, 200),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 0,
            },
            ShapeEdge {
                start: (0, 200),
                end: (0, 0),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 0,
            },
        ],
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        // 1. Validation functions
        checks.push(check(
            "validate_hierarchy_depth",
            validate_hierarchy_depth(10).is_ok() && validate_hierarchy_depth(65).is_err(),
        ));
        checks.push(check(
            "validate_display_depth",
            validate_display_depth(1).is_ok() && validate_display_depth(100_000).is_err(),
        ));
        checks.push(check(
            "validate_morph_ratio",
            validate_morph_ratio(1000).is_ok() && validate_morph_ratio(70_000).is_err(),
        ));

        // 2. MovieClip timeline & hierarchy
        let mut clip = MovieClip::new(10);
        checks.push(check("clip_initial_frame", clip.clip_current_frame() == 1));
        clip.clip_next_frame();
        checks.push(check("clip_next_frame", clip.clip_current_frame() == 2));
        clip.clip_prev_frame();
        checks.push(check("clip_prev_frame", clip.clip_current_frame() == 1));
        clip.clip_stop();
        checks.push(check(
            "clip_stop",
            clip.clip_play_state() == PlayState::Stop,
        ));
        clip.clip_play();
        checks.push(check(
            "clip_play",
            clip.clip_play_state() == PlayState::Play,
        ));
        let _ = clip.clip_add_frame_label("start", 1);
        checks.push(check(
            "clip_frame_by_label",
            clip.clip_frame_by_label("start") == Some(1),
        ));
        let _ = clip.clip_goto_and_stop(FrameTarget::Index(5));
        checks.push(check("clip_goto_and_stop", clip.clip_current_frame() == 5));

        // 3. Child display slot management
        let shape = create_sample_shape();
        let slot = DisplaySlot::new(1, DisplayObject::Shape(shape.clone()));
        let ins_ok = clip.clip_insert_slot(slot).is_ok();
        checks.push(check(
            "clip_insert_slot",
            ins_ok && clip.clip_has_child_at(1),
        ));
        checks.push(check("clip_child_count", clip.clip_child_count() == 1));

        // 4. TextField & variable binding
        let mut tf = TextField::new(10.0, 10.0, 100.0, 20.0, "Hello");
        tf.field_set_bound_variable(Some("score".to_string()));
        checks.push(check(
            "field_bound_variable",
            tf.field_bound_variable() == Some("score"),
        ));
        let sync_ok = tf.field_update_from_variable("score", "42");
        checks.push(check(
            "field_update_from_variable",
            sync_ok && tf.field_text_content() == "42",
        ));

        // 5. MorphShape interpolation
        let shape2 = create_sample_shape();
        let mut morph = MorphShape::new(shape, shape2);
        morph.morph_set_ratio_val(32768);
        let eval_res = morph.morph_evaluate_current();
        checks.push(check(
            "morph_evaluate_current",
            eval_res.is_ok() && morph.morph_current_ratio() == 32768,
        ));

        // 6. SimpleButton interaction
        let mut btn = SimpleButton::new();
        btn.button_set_up_record(DisplayObject::Text(tf.clone()));
        checks.push(check(
            "button_initial_state",
            btn.button_current_state() == ButtonState::Up,
        ));
        let move_ev = btn.button_process_mouse_move(true);
        checks.push(check(
            "button_process_mouse_move",
            move_ev == Some(ButtonEvent::RollOver)
                && btn.button_current_state() == ButtonState::Over,
        ));
        let down_ev = btn.button_process_mouse_down(true);
        checks.push(check(
            "button_process_mouse_down",
            down_ev == Some(ButtonEvent::Press) && btn.button_current_state() == ButtonState::Down,
        ));
        let up_ev = btn.button_process_mouse_up(true);
        checks.push(check(
            "button_process_mouse_up",
            up_ev == Some(ButtonEvent::Release) && btn.button_current_state() == ButtonState::Over,
        ));

        // 7. Stage & event dispatch
        let mut stage = Stage::new(320.0, 240.0, 24.0);
        let advance_ev = stage.stage_advance_frame();
        checks.push(check(
            "stage_advance_frame",
            advance_ev.is_ok() && advance_ev.unwrap() == vec![DisplayListEvent::EnterFrame],
        ));
        let mouse_ev = stage.stage_handle_mouse_move(50.0, 50.0);
        checks.push(check("stage_handle_mouse_move", !mouse_ev.is_empty()));

        // 8. Lowering to PaintScene & scene diffing
        let scene1 = stage.stage_lower_to_scene();
        checks.push(check("stage_lower_to_scene", scene1.is_ok()));
        let mut next_stage = stage.clone();
        next_stage
            .stage_root_clip_mut()
            .clip_insert_slot(DisplaySlot::new(2, DisplayObject::Text(tf)))
            .unwrap();
        let scene2 = next_stage.stage_lower_to_scene().unwrap();

        let mut ops = [PatchOp {
            kind: flash_display_list::PatchKind::Added,
            old_index: 0,
            new_index: 0,
        }; 16];
        let diff_res = diff_paint_scene_into(&scene1.unwrap(), &scene2, &mut ops);
        checks.push(check("diff_paint_scene_into", diff_res.is_ok()));

        // 9. Tag parsing
        let place2_data = [0x06, 0x01, 0x00, 0x0a, 0x00, 0x00]; // depth 1, char 10
        let p2_res = parse_place_object2(&place2_data);
        checks.push(check("parse_place_object2", p2_res.is_ok()));

        let rem2_data = [0x01, 0x00]; // depth 1
        let r2_res = parse_remove_object2(&rem2_data);
        checks.push(check(
            "parse_remove_object2",
            r2_res.is_ok() && r2_res.unwrap().depth == 1,
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/flash-display-list",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
