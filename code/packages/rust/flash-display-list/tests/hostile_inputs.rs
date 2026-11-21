//! Hostile-input security regression and fuzz tests for `flash-display-list` (SWF05).
//!
//! Covers:
//! - `NaN` / `±Inf` / extremal `SwfMatrix` transforms in lowering
//! - Deep `MovieClip` hierarchy `> MAX_HIERARCHY_DEPTH` (`64`) rejection
//! - Invalid display depths, clip depths, and morph ratios + extremal `MorphShape` twips
//! - CWE-400 bounded null-terminated instance/class names in `PlaceObject2`/`PlaceObject3`
//! - Seeded 1,000-case malformed `PlaceObject{,2,3}` / `RemoveObject{,2}` fuzz campaign

#![forbid(unsafe_code)]

use flash_display_list::{
    lower_movie_clip_to_scene, parse_display_list_tag, parse_place_object, parse_place_object2,
    parse_place_object3, parse_remove_object, parse_remove_object2, swf_matrix_to_affine,
    validate_display_depth, validate_hierarchy_depth, validate_morph_ratio, DisplayListError,
    DisplayObject, DisplaySlot, MorphShape, MovieClip, MAX_DEPTH, MAX_HIERARCHY_DEPTH, MIN_DEPTH,
};
use swf_container::{SwfColorTransform, SwfMatrix, SwfRect};
use swf_shape_to_paint::{ParsedShape, ShapeEdge};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound
    }
}

fn empty_shape(xmin: i32, xmax: i32, ymin: i32, ymax: i32) -> ParsedShape {
    ParsedShape {
        tag_code: 2,
        shape_id: 1,
        bounds: SwfRect::create_rect(xmin, xmax, ymin, ymax),
        edge_bounds: None,
        uses_non_zero_winding: false,
        fill_styles: Vec::new(),
        line_styles: Vec::new(),
        edges: vec![ShapeEdge {
            start: (xmin, ymin),
            end: (xmax, ymax),
            control: None,
            fill_style0: 0,
            fill_style1: 0,
            line_style: 0,
        }],
    }
}

#[test]
fn hostile_nan_inf_and_extremal_transforms_no_panics() {
    let hostile_matrices = [
        SwfMatrix {
            scale_x: f64::NAN,
            scale_y: f64::NAN,
            rotate_skew_0: f64::INFINITY,
            rotate_skew_1: f64::NEG_INFINITY,
            translate_x: i32::MAX,
            translate_y: i32::MIN,
        },
        SwfMatrix {
            scale_x: f64::MAX,
            scale_y: -f64::MAX,
            rotate_skew_0: f64::MIN_POSITIVE * 0.5,
            rotate_skew_1: -0.0,
            translate_x: i32::MIN,
            translate_y: i32::MAX,
        },
    ];

    for matrix in hostile_matrices {
        let _affine = swf_matrix_to_affine(&matrix);
        let mut root = MovieClip::new(1);
        let mut slot = DisplaySlot::new(1, DisplayObject::Clip(MovieClip::new(1)));
        slot.transform = matrix;
        slot.color_transform = SwfColorTransform {
            r_mult: i32::MIN,
            g_mult: i32::MAX,
            b_mult: i32::MIN,
            a_mult: i32::MAX,
            r_add: i32::MIN,
            g_add: i32::MAX,
            b_add: i32::MIN,
            a_add: i32::MAX,
        };
        root.clip_insert_slot(slot).unwrap();
        let _ = lower_movie_clip_to_scene(&root, 550.0, 400.0, "#ffffff");
    }
}

#[test]
fn hostile_deep_hierarchy_over_64_rejected() {
    assert_eq!(validate_hierarchy_depth(MAX_HIERARCHY_DEPTH), Ok(true));
    assert_eq!(
        validate_hierarchy_depth(MAX_HIERARCHY_DEPTH + 1),
        Err(DisplayListError::HierarchyDepthExceeded)
    );

    // Build a 70-level deep nested MovieClip chain and assert lowering rejects it cleanly.
    let mut current = MovieClip::new(1);
    for _ in 0..70 {
        let mut parent = MovieClip::new(1);
        let slot = DisplaySlot::new(1, DisplayObject::Clip(current));
        parent.clip_insert_slot(slot).unwrap();
        current = parent;
    }

    let err = lower_movie_clip_to_scene(&current, 550.0, 400.0, "#ffffff")
        .expect_err("hierarchy > MAX_HIERARCHY_DEPTH must fail");
    assert_eq!(err, DisplayListError::HierarchyDepthExceeded);
}

#[test]
fn hostile_invalid_depths_and_morph_ratios_and_extremal_morph() {
    assert_eq!(validate_display_depth(MIN_DEPTH), Ok(true));
    assert_eq!(validate_display_depth(MAX_DEPTH), Ok(true));
    assert_eq!(
        validate_display_depth(MIN_DEPTH - 1),
        Err(DisplayListError::InvalidDepth)
    );
    assert_eq!(
        validate_display_depth(MAX_DEPTH + 1),
        Err(DisplayListError::InvalidDepth)
    );

    let mut mc = MovieClip::new(1);
    assert_eq!(
        mc.clip_insert_slot(DisplaySlot::new(
            MIN_DEPTH - 1,
            DisplayObject::Clip(MovieClip::new(1))
        )),
        Err(DisplayListError::InvalidDepth)
    );
    let mut bad_clip = DisplaySlot::new(10, DisplayObject::Clip(MovieClip::new(1)));
    bad_clip.clip_depth = Some(10); // must be strictly greater than slot.depth
    assert_eq!(
        mc.clip_insert_slot(bad_clip),
        Err(DisplayListError::InvalidClipDepth)
    );

    assert_eq!(validate_morph_ratio(0), Ok(true));
    assert_eq!(validate_morph_ratio(65535), Ok(true));
    assert_eq!(
        validate_morph_ratio(65536),
        Err(DisplayListError::InvalidRatio)
    );

    let start = empty_shape(i32::MIN, i32::MAX, i32::MIN, i32::MAX);
    let end = empty_shape(i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    let morph = MorphShape::new(start, end);
    let mid = morph.morph_evaluate_at_ratio(32768).unwrap();
    assert_eq!(mid.edges.len(), 1);
    let _ = morph.morph_evaluate_at_ratio(u16::MAX).unwrap();
}

#[test]
fn hostile_oversized_cstring_name_capped_cwe400() {
    // PlaceObject2 (tag 26): flags=0x20 (has_name), depth=1, followed by 5000 non-zero bytes + 0
    let mut po2 = vec![0x20, 0x01, 0x00];
    po2.extend(std::iter::repeat_n(b'A', 5000));
    po2.push(0);
    assert_eq!(
        parse_place_object2(&po2),
        Err(DisplayListError::CorruptedData)
    );

    // PlaceObject3 (tag 70): flags1=0x20 (has_name), flags2=0x00, depth=1, 5000 'B's + 0
    let mut po3 = vec![0x20, 0x00, 0x01, 0x00];
    po3.extend(std::iter::repeat_n(b'B', 5000));
    po3.push(0);
    assert_eq!(
        parse_place_object3(&po3),
        Err(DisplayListError::CorruptedData)
    );

    // PlaceObject3 class_name (flags2=0x08 has_class_name) capped too
    let mut po3_cls = vec![0x00, 0x08, 0x01, 0x00];
    po3_cls.extend(std::iter::repeat_n(b'C', 5000));
    po3_cls.push(0);
    assert_eq!(
        parse_place_object3(&po3_cls),
        Err(DisplayListError::CorruptedData)
    );
}

#[test]
fn hostile_tag_parse_fuzz_1000_cases_no_panics() {
    let mut rng = SplitMix64(0x5F05_FEED_FACE_C0DE);
    let tag_codes = [4u16, 5, 26, 28, 70, 0, 999];

    for i in 0..1_000 {
        let len = rng.next_usize(96);
        let buf: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        let code = tag_codes[i % tag_codes.len()];

        let _ = parse_display_list_tag(code, &buf);
        let _ = parse_place_object(&buf);
        let _ = parse_place_object2(&buf);
        let _ = parse_place_object3(&buf);
        let _ = parse_remove_object(&buf);
        let _ = parse_remove_object2(&buf);
    }
}
