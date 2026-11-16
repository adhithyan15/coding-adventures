//! Comprehensive conformance test suite for `flash-display-list` (`SWF05`).

#![deny(unsafe_code)]

use flash_display_list::{
    diff_paint_scene_into, lower_movie_clip_to_scene, parse_display_list_tag, parse_place_object,
    parse_place_object2, parse_remove_object2, validate_display_depth, validate_hierarchy_depth,
    validate_morph_ratio, ButtonEvent, ButtonState, DisplayListError, DisplayListEvent,
    DisplayListTag, DisplayObject, DisplaySlot, FrameTarget, MorphShape, MovieClip,
    PaintInstruction, PatchKind, PatchOp, PlayState, SimpleButton, Stage, TextField, MAX_DEPTH,
    MAX_HIERARCHY_DEPTH, MIN_DEPTH,
};
use swf_container::SwfRect;
use swf_shape_to_paint::{
    ColorRgba, ParsedShape, ShapeEdge, SwfCapStyle, SwfFillStyle, SwfJoinStyle, SwfLineStyle,
};

fn make_test_shape(id: u16, fill_color: ColorRgba) -> ParsedShape {
    ParsedShape {
        tag_code: 2,
        shape_id: id,
        bounds: SwfRect::create_rect(0, 400, 0, 400),
        edge_bounds: None,
        uses_non_zero_winding: false,
        fill_styles: vec![SwfFillStyle::Solid(fill_color)],
        line_styles: vec![SwfLineStyle {
            width_twips: 20,
            color: fill_color,
            cap_style: SwfCapStyle::Round,
            join_style: SwfJoinStyle::Round,
            miter_limit: 3.0,
            no_close: false,
            fill_style: None,
        }],
        edges: vec![
            ShapeEdge {
                start: (0, 0),
                end: (400, 0),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 1,
            },
            ShapeEdge {
                start: (400, 0),
                end: (400, 400),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 1,
            },
            ShapeEdge {
                start: (400, 400),
                end: (0, 400),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 1,
            },
            ShapeEdge {
                start: (0, 400),
                end: (0, 0),
                control: None,
                fill_style0: 0,
                fill_style1: 1,
                line_style: 1,
            },
        ],
    }
}

#[test]
fn test_warrant_validation_bounds() {
    assert_eq!(validate_hierarchy_depth(0), Ok(true));
    assert_eq!(validate_hierarchy_depth(64), Ok(true));
    assert_eq!(
        validate_hierarchy_depth(65),
        Err(DisplayListError::HierarchyDepthExceeded)
    );

    assert_eq!(validate_display_depth(MIN_DEPTH), Ok(true));
    assert_eq!(validate_display_depth(MAX_DEPTH), Ok(true));
    assert_eq!(validate_display_depth(0), Ok(true));
    assert_eq!(
        validate_display_depth(MIN_DEPTH - 1),
        Err(DisplayListError::InvalidDepth)
    );
    assert_eq!(
        validate_display_depth(MAX_DEPTH + 1),
        Err(DisplayListError::InvalidDepth)
    );

    assert_eq!(validate_morph_ratio(0), Ok(true));
    assert_eq!(validate_morph_ratio(65535), Ok(true));
    assert_eq!(
        validate_morph_ratio(65536),
        Err(DisplayListError::InvalidRatio)
    );
}

#[test]
fn test_movie_clip_timeline_fsm() {
    let mut clip = MovieClip::new(5);
    assert_eq!(clip.clip_current_frame(), 1);
    assert_eq!(clip.clip_total_frames(), 5);
    assert_eq!(clip.clip_play_state(), PlayState::Play);

    // Timeline stepping and looping
    clip.clip_step_timeline();
    assert_eq!(clip.clip_current_frame(), 2);
    clip.clip_step_timeline();
    assert_eq!(clip.clip_current_frame(), 3);
    clip.clip_step_timeline();
    assert_eq!(clip.clip_current_frame(), 4);
    clip.clip_step_timeline();
    assert_eq!(clip.clip_current_frame(), 5);
    clip.clip_step_timeline();
    assert_eq!(clip.clip_current_frame(), 1); // Loops to 1

    // Stop and Play
    clip.clip_stop();
    assert_eq!(clip.clip_play_state(), PlayState::Stop);
    clip.clip_step_timeline();
    assert_eq!(clip.clip_current_frame(), 1); // Does not advance while stopped

    clip.clip_play();
    assert_eq!(clip.clip_play_state(), PlayState::Play);

    // Frame labels
    assert!(clip.clip_add_frame_label("intro", 1).is_ok());
    assert!(clip.clip_add_frame_label("boss", 4).is_ok());
    assert!(clip.clip_add_frame_label("invalid", 6).is_err());

    assert_eq!(clip.clip_frame_by_label("boss"), Some(4));
    assert_eq!(clip.clip_frame_by_label("missing"), None);

    // GotoAndStop / GotoAndPlay
    assert!(clip
        .clip_goto_and_stop(FrameTarget::Label("boss".to_string()))
        .is_ok());
    assert_eq!(clip.clip_current_frame(), 4);
    assert_eq!(clip.clip_play_state(), PlayState::Stop);

    assert!(clip.clip_goto_and_play(FrameTarget::Index(2)).is_ok());
    assert_eq!(clip.clip_current_frame(), 2);
    assert_eq!(clip.clip_play_state(), PlayState::Play);

    assert!(clip.clip_goto_and_play(FrameTarget::Index(0)).is_err());
    assert!(clip.clip_goto_and_play(FrameTarget::Index(10)).is_err());
}

#[test]
fn test_movie_clip_depth_slots() {
    let mut clip = MovieClip::new(1);
    let shape1 = make_test_shape(1, ColorRgba::rgba(255, 0, 0, 255));
    let shape2 = make_test_shape(2, ColorRgba::rgba(0, 255, 0, 255));

    let slot1 = DisplaySlot::new(10, DisplayObject::Shape(shape1));
    let slot2 = DisplaySlot::new(5, DisplayObject::Shape(shape2));

    assert!(clip.clip_insert_slot(slot1).is_ok());
    assert!(clip.clip_insert_slot(slot2).is_ok());
    assert_eq!(clip.clip_child_count(), 2);
    assert!(clip.clip_has_child_at(10));
    assert!(clip.clip_has_child_at(5));
    assert!(!clip.clip_has_child_at(1));

    // Ascending depth order
    let slots = clip.clip_slots_depth_order();
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].depth, 5);
    assert_eq!(slots[1].depth, 10);

    // Removal
    let removed = clip.clip_remove_slot(5);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().depth, 5);
    assert_eq!(clip.clip_child_count(), 1);
    assert!(!clip.clip_has_child_at(5));
}

#[test]
fn test_hierarchy_depth_limit() {
    // Build 63-level hierarchy from the bottom up
    let mut current = MovieClip::new(1);
    for d in (1..MAX_HIERARCHY_DEPTH).rev() {
        let mut parent = MovieClip::new(1);
        parent
            .clip_insert_slot(DisplaySlot::new(d as i32, DisplayObject::Clip(current)))
            .unwrap();
        current = parent;
    }
    let mut root = current;

    // Step at depth 63 must succeed
    assert!(root.clip_step_hierarchy(0).is_ok());

    // Build 65-level hierarchy from bottom up to exceed MAX_HIERARCHY_DEPTH
    let mut deep = MovieClip::new(1);
    for d in (0..=MAX_HIERARCHY_DEPTH).rev() {
        let mut parent = MovieClip::new(1);
        parent
            .clip_insert_slot(DisplaySlot::new(d as i32, DisplayObject::Clip(deep)))
            .unwrap();
        deep = parent;
    }

    // Now hierarchy depth must trigger error
    assert_eq!(
        deep.clip_step_hierarchy(0),
        Err(DisplayListError::HierarchyDepthExceeded)
    );
}

#[test]
fn test_simple_button_interaction() {
    let mut button = SimpleButton::new();
    let up_tf = TextField::new(0.0, 0.0, 80.0, 24.0, "Up");
    let over_tf = TextField::new(0.0, 0.0, 80.0, 24.0, "Over");
    let down_tf = TextField::new(0.0, 0.0, 80.0, 24.0, "Down");
    let hit_shape = make_test_shape(10, ColorRgba::rgba(0, 0, 0, 255));

    button.button_set_up_record(DisplayObject::Text(up_tf));
    button.button_set_over_record(DisplayObject::Text(over_tf));
    button.button_set_down_record(DisplayObject::Text(down_tf));
    button.button_set_hit_record(DisplayObject::Shape(hit_shape));

    assert_eq!(button.button_current_state(), ButtonState::Up);
    assert_eq!(
        button
            .button_active_record()
            .unwrap()
            .obj_as_text()
            .unwrap()
            .field_text_content(),
        "Up"
    );

    // Roll over
    let ev = button.button_process_mouse_move(true);
    assert_eq!(ev, Some(ButtonEvent::RollOver));
    assert_eq!(button.button_current_state(), ButtonState::Over);
    assert_eq!(
        button
            .button_active_record()
            .unwrap()
            .obj_as_text()
            .unwrap()
            .field_text_content(),
        "Over"
    );

    // Press down
    let ev = button.button_process_mouse_down(true);
    assert_eq!(ev, Some(ButtonEvent::Press));
    assert_eq!(button.button_current_state(), ButtonState::Down);
    assert_eq!(
        button
            .button_active_record()
            .unwrap()
            .obj_as_text()
            .unwrap()
            .field_text_content(),
        "Down"
    );

    // Release inside
    let ev = button.button_process_mouse_up(true);
    assert_eq!(ev, Some(ButtonEvent::Release));
    assert_eq!(button.button_current_state(), ButtonState::Over);

    // Roll out
    let ev = button.button_process_mouse_move(false);
    assert_eq!(ev, Some(ButtonEvent::RollOut));
    assert_eq!(button.button_current_state(), ButtonState::Up);

    // Press and drag outside release
    button.button_process_mouse_move(true);
    button.button_process_mouse_down(true);
    button.button_process_mouse_move(false);
    let ev = button.button_process_mouse_up(false);
    assert_eq!(ev, Some(ButtonEvent::ReleaseOutside));
    assert_eq!(button.button_current_state(), ButtonState::Up);

    // Disabled state
    button.button_set_enabled(false);
    assert_eq!(button.button_process_mouse_move(true), None);
    assert_eq!(button.button_current_state(), ButtonState::Up);
}

#[test]
fn test_text_field_variable_binding() {
    let mut tf = TextField::new(10.0, 20.0, 150.0, 30.0, "Player");
    tf.field_set_bound_variable(Some("player_name".to_string()));
    tf.field_set_border(true, ColorRgba::rgba(0, 0, 0, 255));
    tf.field_set_background_fill(Some(ColorRgba::rgba(255, 255, 255, 255)));

    assert_eq!(tf.field_text_content(), "Player");
    assert_eq!(tf.field_bound_variable(), Some("player_name"));
    assert!(tf.field_has_border());
    assert!(tf.field_background_fill().is_some());

    // Variable sync
    assert!(tf.field_update_from_variable("player_name", "Hero"));
    assert_eq!(tf.field_text_content(), "Hero");

    // Unrelated variable does not mutate
    assert!(!tf.field_update_from_variable("score", "9000"));
    assert_eq!(tf.field_text_content(), "Hero");
}

#[test]
fn test_morph_shape_interpolation() {
    let start = make_test_shape(1, ColorRgba::rgba(0, 0, 0, 255));
    let end = make_test_shape(2, ColorRgba::rgba(255, 255, 255, 255));

    let mut morph = MorphShape::new(start, end);
    assert_eq!(morph.morph_current_ratio(), 0);

    // Ratio 0 -> evaluates to start shape colors
    let s0 = morph.morph_evaluate_at_ratio(0).unwrap();
    if let SwfFillStyle::Solid(c) = s0.fill_styles[0] {
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    } else {
        panic!("expected solid fill");
    }

    // Ratio 65535 -> evaluates to end shape colors
    let s1 = morph.morph_evaluate_at_ratio(65535).unwrap();
    if let SwfFillStyle::Solid(c) = s1.fill_styles[0] {
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
    } else {
        panic!("expected solid fill");
    }

    // Midpoint ratio 32768
    morph.morph_set_ratio_val(32768);
    let s_mid = morph.morph_evaluate_current().unwrap();
    if let SwfFillStyle::Solid(c) = s_mid.fill_styles[0] {
        assert!((127..=128).contains(&c.r));
        assert!((127..=128).contains(&c.g));
        assert!((127..=128).contains(&c.b));
    } else {
        panic!("expected solid fill");
    }
}

#[test]
fn test_place_remove_tag_parsing() {
    // 1. PlaceObject (Tag 4): char 2, depth 1, identity matrix
    let po_data = [
        0x02, 0x00, // char 2
        0x01, 0x00, // depth 1
        0x00, // matrix: no scale, no rotate, 0 nbits translate
    ];
    let po = parse_place_object(&po_data).unwrap();
    assert_eq!(po.depth, 1);
    assert_eq!(po.character_id, Some(2));
    assert!(po.matrix.is_some());

    // 2. PlaceObject2 (Tag 26): has_character | has_matrix, depth 5, char 42
    let po2_data = [
        0x06, // flags: has_move=false, has_character=true, has_matrix=true
        0x05, 0x00, // depth 5
        0x2a, 0x00, // char 42
        0x00, // matrix
    ];
    let po2 = parse_place_object2(&po2_data).unwrap();
    assert_eq!(po2.depth, 5);
    assert_eq!(po2.character_id, Some(42));
    assert!(!po2.is_move);

    // 3. RemoveObject2 (Tag 28): depth 5
    let ro2_data = [0x05, 0x00];
    let ro2 = parse_remove_object2(&ro2_data).unwrap();
    assert_eq!(ro2.depth, 5);
    assert_eq!(ro2.character_id, None);

    // 4. Dispatcher
    let tag = parse_display_list_tag(28, &ro2_data).unwrap();
    assert_eq!(tag, DisplayListTag::Remove(ro2));
}

#[test]
fn test_stencil_mask_clipping_lowering() {
    let mut clip = MovieClip::new(1);

    // Slot 1: Mask shape with clip_depth = Some(3)
    let mask_shape = make_test_shape(1, ColorRgba::rgba(0, 0, 0, 255));
    let mut mask_slot = DisplaySlot::new(1, DisplayObject::Shape(mask_shape));
    mask_slot.clip_depth = Some(3);
    clip.clip_insert_slot(mask_slot).unwrap();

    // Slot 2: Child A (masked)
    let child_a = make_test_shape(2, ColorRgba::rgba(255, 0, 0, 255));
    clip.clip_insert_slot(DisplaySlot::new(2, DisplayObject::Shape(child_a)))
        .unwrap();

    // Slot 3: Child B (masked)
    let child_b = make_test_shape(3, ColorRgba::rgba(0, 255, 0, 255));
    clip.clip_insert_slot(DisplaySlot::new(3, DisplayObject::Shape(child_b)))
        .unwrap();

    // Slot 4: Child C (unmasked, outside clip range)
    let child_c = make_test_shape(4, ColorRgba::rgba(0, 0, 255, 255));
    clip.clip_insert_slot(DisplaySlot::new(4, DisplayObject::Shape(child_c)))
        .unwrap();

    let scene = lower_movie_clip_to_scene(&clip, 320.0, 240.0, "#ffffff").unwrap();

    // Instructions should contain 1 Clip instruction (wrapping children at depth 2 and 3)
    // and 1 Group instruction for Child C at depth 4!
    assert_eq!(scene.instructions.len(), 2);

    match &scene.instructions[0] {
        PaintInstruction::Clip(clip_inst) => {
            assert!(clip_inst.path.is_some());
            // Inside the clip instruction are children 2 and 3
            assert_eq!(clip_inst.children.len(), 2);
        }
        other => panic!("expected PaintInstruction::Clip, got {:?}", other),
    }

    match &scene.instructions[1] {
        PaintInstruction::Group(group_inst) => {
            assert_eq!(group_inst.base.id, Some("root/slot_4".to_string()));
        }
        other => panic!("expected PaintInstruction::Group, got {:?}", other),
    }
}

#[test]
fn test_stage_event_dispatch_and_retained_diffing() {
    let mut stage = Stage::new(640.0, 480.0, 30.0);
    assert_eq!(stage.stage_dimensions(), (640.0, 480.0));
    assert_eq!(stage.stage_frame_rate(), 30.0);

    // Add button at depth 1
    let mut btn = SimpleButton::new();
    let hit_shape = make_test_shape(10, ColorRgba::rgba(0, 0, 0, 255));
    btn.button_set_hit_record(DisplayObject::Shape(hit_shape));
    let slot = DisplaySlot::new(1, DisplayObject::Button(btn));
    stage.stage_root_clip_mut().clip_insert_slot(slot).unwrap();

    // Initial frame lowering
    let frame1_scene = stage.stage_lower_to_scene().unwrap();

    // Mouse move outside hit area (x=1000, y=1000)
    let evs = stage.stage_handle_mouse_move(1000.0, 1000.0);
    assert_eq!(evs.len(), 1); // Only MouseMove

    // Mouse move inside hit area (x=10, y=10)
    let evs = stage.stage_handle_mouse_move(10.0, 10.0);
    assert_eq!(evs.len(), 2); // MouseMove + ButtonAction(RollOver)
    assert!(matches!(
        evs[1],
        DisplayListEvent::ButtonAction {
            depth: 1,
            event: ButtonEvent::RollOver,
            ..
        }
    ));

    // Mouse down inside hit area
    let evs = stage.stage_handle_mouse_down();
    assert_eq!(evs.len(), 2); // MouseDown + ButtonAction(Press)

    // Mouse up inside hit area
    let evs = stage.stage_handle_mouse_up();
    assert_eq!(evs.len(), 2); // MouseUp + ButtonAction(Release)

    // Advance frame
    let advance_evs = stage.stage_advance_frame().unwrap();
    assert_eq!(advance_evs, vec![DisplayListEvent::EnterFrame]);

    // Mutate scene: add object at depth 2
    let shape2 = make_test_shape(20, ColorRgba::rgba(255, 0, 0, 255));
    stage
        .stage_root_clip_mut()
        .clip_insert_slot(DisplaySlot::new(2, DisplayObject::Shape(shape2)))
        .unwrap();

    let frame2_scene = stage.stage_lower_to_scene().unwrap();

    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 16];

    let summary = diff_paint_scene_into(&frame1_scene, &frame2_scene, &mut ops).unwrap();
    assert_eq!(summary.added, 1);
    assert_eq!(summary.removed, 0);
    assert_eq!(summary.modified, 0);
    assert_eq!(summary.unchanged, 0); // frame1 button has no active display object, so 0 insts in frame 1
}
