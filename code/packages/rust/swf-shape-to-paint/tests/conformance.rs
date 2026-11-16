//! Conformance tests for `swf-shape-to-paint` (`SWF02`).
//!
//! Validates:
//! - Formal warrant functions (`validate_shape_tag`, `validate_twips`, `validate_style_count`)
//! - TWIPS to pixel coordinate conversions and overflow guards
//! - Parsing `DefineShape`, `DefineShape2`, `DefineShape3`, `DefineShape4`
//! - Solid fills, linear/radial/focal gradients, bitmap fills, and line styles
//! - Dual-fill edge loop reconstruction (`FillStyle0` / `FillStyle1`) with shared edges
//! - Inner island cutouts with `FillRule::EvenOdd` and `FillRule::NonZero`
//! - Lowering to `PaintScene` and `PaintInstruction`s using `bezier2d`
//! - Security boundaries and anti-exhaustion caps (CWE-20/125/190/369/400/787)

#![deny(unsafe_code)]

use paint_instructions::{
    FillRule, GradientKind, PaintInstruction, PathCommand, StrokeCap, StrokeJoin,
};
use swf_container::SwfRect;
use swf_shape_to_paint::{
    checked_twips_add, checked_twips_point_add, evaluate_quad_edge, flatten_quad_edge,
    lower_shape_to_paint_scene, parse_define_shape, pixels_to_twips_i32, quad_edge_bounding_box,
    reconstruct_fill_contours, reconstruct_stroke_runs, twips_point_to_pixels, twips_to_pixels_f64,
    validate_shape_tag, validate_style_count, validate_twips, ColorRgba, PureCapabilities,
    ShapeEdge, ShapeError, SwfCapStyle, SwfJoinStyle, MAX_SAFE_TWIPS, MIN_SAFE_TWIPS,
    PIXELS_PER_TWIP, TWIPS_PER_PIXEL,
};

// ============================================================================
// BitWriter helper for synthesizing SWF binary tags in unit tests
// ============================================================================

struct BitWriter {
    bytes: Vec<u8>,
    bit_pos: usize, // 0..8
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_pos: 0,
        }
    }

    fn write_ub(&mut self, val: u32, n: usize) {
        for i in (0..n).rev() {
            let bit = ((val >> i) & 1) as u8;
            if self.bit_pos == 0 {
                self.bytes.push(0);
            }
            let byte_idx = self.bytes.len() - 1;
            self.bytes[byte_idx] |= bit << (7 - self.bit_pos);
            self.bit_pos = (self.bit_pos + 1) % 8;
        }
    }

    fn write_sb(&mut self, val: i32, n: usize) {
        let mask = if n == 32 {
            0xFFFF_FFFF
        } else {
            (1u32 << n) - 1
        };
        let u = (val as u32) & mask;
        self.write_ub(u, n);
    }

    fn align_to_byte(&mut self) {
        if self.bit_pos > 0 {
            self.bit_pos = 0;
        }
    }

    fn write_ui8(&mut self, val: u8) {
        self.align_to_byte();
        self.bytes.push(val);
    }

    fn write_ui16_le(&mut self, val: u16) {
        self.align_to_byte();
        self.bytes.extend_from_slice(&val.to_le_bytes());
    }

    fn write_rect(&mut self, xmin: i32, xmax: i32, ymin: i32, ymax: i32) {
        let max_val = xmin
            .abs()
            .max(xmax.abs())
            .max(ymin.abs())
            .max(ymax.abs())
            .max(1);
        let mut bits = 0;
        let mut temp = max_val;
        while temp > 0 {
            bits += 1;
            temp >>= 1;
        }
        let nbits = bits + 2;
        self.write_ub(nbits as u32, 5);
        self.write_sb(xmin, nbits);
        self.write_sb(xmax, nbits);
        self.write_sb(ymin, nbits);
        self.write_sb(ymax, nbits);
        self.align_to_byte();
    }

    fn write_matrix_identity(&mut self) {
        self.write_ub(0, 1); // has_scale = 0
        self.write_ub(0, 1); // has_rotate = 0
        self.write_ub(0, 5); // n_translate = 0
        self.align_to_byte();
    }

    fn finish(mut self) -> Vec<u8> {
        self.align_to_byte();
        self.bytes
    }
}

// ============================================================================
// Warrant and Basic Conformance Tests
// ============================================================================

#[test]
fn test_warrant_validation_functions() {
    // validate_shape_tag
    assert_eq!(validate_shape_tag(2), Ok(true));
    assert_eq!(validate_shape_tag(22), Ok(true));
    assert_eq!(validate_shape_tag(32), Ok(true));
    assert_eq!(validate_shape_tag(83), Ok(true));
    assert_eq!(validate_shape_tag(0), Err(ShapeError::InvalidTag));
    assert_eq!(validate_shape_tag(10), Err(ShapeError::InvalidTag));

    // validate_twips
    assert_eq!(validate_twips(0), Ok(true));
    assert_eq!(validate_twips(1000), Ok(true));
    assert_eq!(validate_twips(MAX_SAFE_TWIPS), Ok(true));
    assert_eq!(validate_twips(MIN_SAFE_TWIPS), Ok(true));
    assert_eq!(
        validate_twips(MAX_SAFE_TWIPS + 1),
        Err(ShapeError::ArithmeticOverflow)
    );
    assert_eq!(
        validate_twips(MIN_SAFE_TWIPS - 1),
        Err(ShapeError::ArithmeticOverflow)
    );

    // validate_style_count
    assert_eq!(validate_style_count(0), Ok(true));
    assert_eq!(validate_style_count(255), Ok(true));
    assert_eq!(validate_style_count(65_535), Ok(true));
    assert_eq!(
        validate_style_count(65_536),
        Err(ShapeError::ExcessiveStyles)
    );
}

#[test]
fn test_twips_math_and_conversions() {
    assert_eq!(twips_to_pixels_f64(20), 1.0);
    assert_eq!(twips_to_pixels_f64(0), 0.0);
    assert_eq!(twips_to_pixels_f64(-200), -10.0);
    assert_eq!(pixels_to_twips_i32(1.0), 20);
    assert_eq!(pixels_to_twips_i32(0.05), 1);
    assert_eq!(twips_point_to_pixels(100, 200), (5.0, 10.0));

    assert_eq!(checked_twips_add(10, 20), Ok(30));
    assert_eq!(
        checked_twips_add(MAX_SAFE_TWIPS, 1),
        Err(ShapeError::ArithmeticOverflow)
    );
    assert_eq!(checked_twips_point_add(10, 20, 5, -5), Ok((15, 15)));

    assert_eq!(TWIPS_PER_PIXEL, 20.0);
    assert_eq!(PIXELS_PER_TWIP, 0.05);
}

#[test]
fn test_capabilities_token() {
    let token = PureCapabilities;
    let copy = token;
    assert_eq!(token, copy);
}

#[test]
fn test_color_rgba_and_styles() {
    let opaque = ColorRgba::rgb(255, 0, 128);
    assert_eq!(opaque.color_to_css_string(), "#ff0080");
    assert_eq!(opaque.color_to_components(), (255, 0, 128, 255));

    let transparent = ColorRgba::rgba(0, 255, 0, 128);
    assert!(transparent
        .color_to_css_string()
        .starts_with("rgba(0, 255, 0,"));

    assert_eq!(SwfCapStyle::Round.cap_to_paint_cap(), StrokeCap::Round);
    assert_eq!(SwfCapStyle::Butt.cap_to_paint_cap(), StrokeCap::Butt);
    assert_eq!(SwfCapStyle::Square.cap_to_paint_cap(), StrokeCap::Square);

    assert_eq!(SwfJoinStyle::Round.join_to_paint_join(), StrokeJoin::Round);
    assert_eq!(SwfJoinStyle::Bevel.join_to_paint_join(), StrokeJoin::Bevel);
    assert_eq!(SwfJoinStyle::Miter.join_to_paint_join(), StrokeJoin::Miter);
}

#[test]
fn test_bezier2d_integration() {
    let edge = ShapeEdge {
        start: (0, 0),
        end: (400, 0),
        control: Some((200, 400)),
        fill_style0: 1,
        fill_style1: 0,
        line_style: 0,
    };

    assert!(edge.is_curved_edge());
    let p_start = evaluate_quad_edge(&edge, 0.0);
    let p_mid = evaluate_quad_edge(&edge, 0.5);
    let p_end = evaluate_quad_edge(&edge, 1.0);

    assert_eq!(p_start, (0.0, 0.0));
    assert_eq!(p_end, (20.0, 0.0));
    assert_eq!(p_mid, (10.0, 10.0));

    let bbox = quad_edge_bounding_box(&edge);
    assert_eq!(bbox.0, 0.0); // min_x
    assert_eq!(bbox.1, 0.0); // min_y
    assert_eq!(bbox.2, 20.0); // max_x
    assert_eq!(bbox.3, 10.0); // max_y

    let poly = flatten_quad_edge(&edge, 0.1).expect("flattening succeeds");
    assert!(poly.len() >= 3);
}

// ============================================================================
// Synthetic Shape Tag Binary Parsing & Lowering Tests
// ============================================================================

#[test]
fn test_define_shape1_parsing_and_lowering() {
    let mut w = BitWriter::new();
    w.write_ui16_le(42); // ShapeId
    w.write_rect(0, 200, 0, 200); // Bounds: 0..10px, 0..10px

    // FillStyleArray: 1 solid RGB fill (red: 255, 0, 0)
    w.write_ui8(1);
    w.write_ui8(0x00); // Solid
    w.write_ui8(255);
    w.write_ui8(0);
    w.write_ui8(0);

    // LineStyleArray: 0
    w.write_ui8(0);

    // NumFillBits = 1, NumLineBits = 0
    w.write_ub(1, 4);
    w.write_ub(0, 4);

    // ShapeRecords:
    // 1. StyleChange: MoveTo (0, 0), FillStyle1 = 1
    w.write_ub(0, 1); // type_flag = 0
    w.write_ub(0b00101, 5); // state_fill_style1 (bit 2) | state_move_to (bit 0)
    w.write_ub(9, 5); // move_bits = 9
    w.write_sb(0, 9);
    w.write_sb(0, 9);
    w.write_ub(1, 1); // fill_style1 = 1

    // 2. StraightEdge: dx = 200, dy = 0 (nbits = 10 -> n - 2 = 8)
    w.write_ub(1, 1); // type_flag = 1
    w.write_ub(1, 1); // straight = 1
    w.write_ub(8, 4); // nbits = 10
    w.write_ub(0, 1); // general_line = 0
    w.write_ub(0, 1); // vert = 0 (horizontal)
    w.write_sb(200, 10);

    // 3. StraightEdge: dx = 0, dy = 200
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(8, 4);
    w.write_ub(0, 1);
    w.write_ub(1, 1); // vert = 1 (vertical)
    w.write_sb(200, 10);

    // 4. StraightEdge: dx = -200, dy = 0
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(8, 4);
    w.write_ub(0, 1);
    w.write_ub(0, 1);
    w.write_sb(-200, 10);

    // 5. StraightEdge: dx = 0, dy = -200
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(8, 4);
    w.write_ub(0, 1);
    w.write_ub(1, 1);
    w.write_sb(-200, 10);

    // 6. EndShapeRecord: 6 zero bits
    w.write_ub(0, 1);
    w.write_ub(0, 5);

    let bytes = w.finish();

    let parsed = parse_define_shape(2, &bytes).expect("parse DefineShape");
    assert_eq!(parsed.shape_id, 42);
    assert_eq!(parsed.parsed_fill_style_count(), 1);
    assert_eq!(parsed.parsed_line_style_count(), 0);
    assert_eq!(parsed.parsed_edge_count(), 4);

    let scene = lower_shape_to_paint_scene(&parsed);
    assert_eq!(scene.width, 10.0);
    assert_eq!(scene.height, 10.0);
    assert_eq!(scene.instructions.len(), 1);

    match &scene.instructions[0] {
        PaintInstruction::Path(path) => {
            assert_eq!(path.fill.as_deref(), Some("#ff0000"));
            assert_eq!(path.fill_rule, Some(FillRule::EvenOdd));
            assert_eq!(path.commands.len(), 6);
            assert_eq!(path.commands[0], PathCommand::MoveTo { x: 0.0, y: 0.0 });
            assert_eq!(path.commands[1], PathCommand::LineTo { x: 10.0, y: 0.0 });
            assert_eq!(path.commands[2], PathCommand::LineTo { x: 10.0, y: 10.0 });
            assert_eq!(path.commands[3], PathCommand::LineTo { x: 0.0, y: 10.0 });
            assert_eq!(path.commands[4], PathCommand::LineTo { x: 0.0, y: 0.0 });
            assert_eq!(path.commands[5], PathCommand::Close);
        }
        _ => panic!("expected Path instruction"),
    }
}

#[test]
fn test_define_shape3_rgba_and_strokes() {
    let mut w = BitWriter::new();
    w.write_ui16_le(99); // ShapeId
    w.write_rect(0, 400, 0, 400); // 0..20px

    // FillStyleArray: 1 RGBA fill (blue with 50% opacity)
    w.write_ui8(1);
    w.write_ui8(0x00); // Solid
    w.write_ui8(0);
    w.write_ui8(0);
    w.write_ui8(255);
    w.write_ui8(128); // alpha 128

    // LineStyleArray: 1 RGBA stroke (black, 40 twips = 2px)
    w.write_ui8(1);
    w.write_ui16_le(40); // 40 twips
    w.write_ui8(0);
    w.write_ui8(0);
    w.write_ui8(0);
    w.write_ui8(255);

    // NumFillBits = 1, NumLineBits = 1
    w.write_ub(1, 4);
    w.write_ub(1, 4);

    // ShapeRecords:
    // MoveTo (0, 0), FillStyle1 = 1, LineStyle = 1
    w.write_ub(0, 1);
    w.write_ub(0b01101, 5); // state_line_style | state_fill_style1 | state_move_to
    w.write_ub(9, 5);
    w.write_sb(0, 9);
    w.write_sb(0, 9);
    w.write_ub(1, 1); // fill_style1 = 1
    w.write_ub(1, 1); // line_style = 1

    // Triangle: (0,0) -> (400,0) -> (0,400) -> (0,0)
    // Edge 1: (0,0) -> (400, 0)
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(9, 4); // nbits = 11
    w.write_ub(0, 1);
    w.write_ub(0, 1);
    w.write_sb(400, 11);

    // Edge 2: (400,0) -> (0, 400) -> dx = -400, dy = 400
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(9, 4);
    w.write_ub(1, 1); // general line
    w.write_sb(-400, 11);
    w.write_sb(400, 11);

    // Edge 3: (0,400) -> (0, 0) -> dx = 0, dy = -400
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(9, 4);
    w.write_ub(0, 1);
    w.write_ub(1, 1); // vert
    w.write_sb(-400, 11);

    // EndShapeRecord
    w.write_ub(0, 1);
    w.write_ub(0, 5);

    let bytes = w.finish();

    let parsed = parse_define_shape(32, &bytes).expect("parse DefineShape3");
    assert_eq!(parsed.shape_id, 99);
    assert_eq!(parsed.parsed_fill_style_count(), 1);
    assert_eq!(parsed.parsed_line_style_count(), 1);

    let scene = lower_shape_to_paint_scene(&parsed);
    // Should have 2 instructions: 1 fill path and 1 stroke path
    assert_eq!(scene.instructions.len(), 2);

    match &scene.instructions[0] {
        PaintInstruction::Path(fill_path) => {
            assert!(fill_path
                .fill
                .as_deref()
                .unwrap()
                .contains("rgba(0, 0, 255"));
            assert_eq!(fill_path.commands.len(), 5);
        }
        _ => panic!("expected fill path"),
    }

    match &scene.instructions[1] {
        PaintInstruction::Path(stroke_path) => {
            assert_eq!(stroke_path.stroke.as_deref(), Some("#000000"));
            assert_eq!(stroke_path.stroke_width, Some(2.0));
            assert_eq!(stroke_path.stroke_cap, Some(StrokeCap::Round));
            assert_eq!(stroke_path.stroke_join, Some(StrokeJoin::Round));
        }
        _ => panic!("expected stroke path"),
    }
}

#[test]
fn test_dual_fill_shared_edge_reconstruction() {
    // Two triangles sharing an internal diagonal edge:
    // Edge A: (0,0) -> (100,0) fill1 = 1
    // Edge B: (100,0) -> (100,100) fill1 = 1
    // Edge C: (0,0) -> (100,100) fill1 = 1, fill0 = 2 (Shared diagonal!)
    // Edge D: (100,100) -> (0,100) fill1 = 2
    // Edge E: (0,100) -> (0,0) fill1 = 2

    let e_a = ShapeEdge {
        start: (0, 0),
        end: (100, 0),
        control: None,
        fill_style0: 0,
        fill_style1: 1,
        line_style: 0,
    };
    let e_b = ShapeEdge {
        start: (100, 0),
        end: (100, 100),
        control: None,
        fill_style0: 0,
        fill_style1: 1,
        line_style: 0,
    };
    let e_c = ShapeEdge {
        start: (0, 0),
        end: (100, 100),
        control: None,
        fill_style0: 1, // Left is fill 1 (Triangle A)
        fill_style1: 2, // Right is fill 2 (Triangle B)
        line_style: 0,
    };
    let e_d = ShapeEdge {
        start: (100, 100),
        end: (0, 100),
        control: None,
        fill_style0: 0,
        fill_style1: 2,
        line_style: 0,
    };
    let e_e = ShapeEdge {
        start: (0, 100),
        end: (0, 0),
        control: None,
        fill_style0: 0,
        fill_style1: 2,
        line_style: 0,
    };

    let edges = vec![e_a, e_b, e_c, e_d, e_e];

    // Reconstruct fill 1:
    let contours_fill1 = reconstruct_fill_contours(&edges, 1);
    assert_eq!(contours_fill1.len(), 1);
    assert!(contours_fill1[0].is_closed);

    // Reconstruct fill 2:
    let contours_fill2 = reconstruct_fill_contours(&edges, 2);
    assert_eq!(contours_fill2.len(), 1);
    assert!(contours_fill2[0].is_closed);
}

#[test]
fn test_inner_island_cutout_reconstruction() {
    // Donut: outer square (0..400) and inner square hole (100..300)
    // Outer square: (0,0) -> (400,0) -> (400,400) -> (0,400) -> (0,0) fill1 = 1
    // Inner square: (100,100) -> (300,100) -> (300,300) -> (100,300) -> (100,100) fill0 = 1 (hole)

    let outer = vec![
        ShapeEdge {
            start: (0, 0),
            end: (400, 0),
            control: None,
            fill_style0: 0,
            fill_style1: 1,
            line_style: 0,
        },
        ShapeEdge {
            start: (400, 0),
            end: (400, 400),
            control: None,
            fill_style0: 0,
            fill_style1: 1,
            line_style: 0,
        },
        ShapeEdge {
            start: (400, 400),
            end: (0, 400),
            control: None,
            fill_style0: 0,
            fill_style1: 1,
            line_style: 0,
        },
        ShapeEdge {
            start: (0, 400),
            end: (0, 0),
            control: None,
            fill_style0: 0,
            fill_style1: 1,
            line_style: 0,
        },
    ];

    let inner = vec![
        ShapeEdge {
            start: (100, 100),
            end: (300, 100),
            control: None,
            fill_style0: 1, // fill 1 on left (island cutout)
            fill_style1: 0,
            line_style: 0,
        },
        ShapeEdge {
            start: (300, 100),
            end: (300, 300),
            control: None,
            fill_style0: 1,
            fill_style1: 0,
            line_style: 0,
        },
        ShapeEdge {
            start: (300, 300),
            end: (100, 300),
            control: None,
            fill_style0: 1,
            fill_style1: 0,
            line_style: 0,
        },
        ShapeEdge {
            start: (100, 300),
            end: (100, 100),
            control: None,
            fill_style0: 1,
            fill_style1: 0,
            line_style: 0,
        },
    ];

    let mut edges = outer;
    edges.extend(inner);

    let contours = reconstruct_fill_contours(&edges, 1);
    // Two closed contours: outer boundary + inner island
    assert_eq!(contours.len(), 2);
    assert!(contours[0].is_closed);
    assert!(contours[1].is_closed);
}

#[test]
fn test_stroke_runs_reconstruction() {
    let e1 = ShapeEdge {
        start: (0, 0),
        end: (100, 0),
        control: None,
        fill_style0: 0,
        fill_style1: 0,
        line_style: 1,
    };
    let e2 = ShapeEdge {
        start: (100, 0),
        end: (200, 100),
        control: None,
        fill_style0: 0,
        fill_style1: 0,
        line_style: 1,
    };
    let e3 = ShapeEdge {
        start: (500, 500),
        end: (600, 600),
        control: None,
        fill_style0: 0,
        fill_style1: 0,
        line_style: 1,
    };

    let edges = vec![e1, e2, e3];
    let runs = reconstruct_stroke_runs(&edges, 1);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].stroke_edge_count(), 2);
    assert_eq!(runs[1].stroke_edge_count(), 1);
}

#[test]
fn test_define_shape4_focal_gradient_and_winding() {
    let mut w = BitWriter::new();
    w.write_ui16_le(7); // ShapeId
    w.write_rect(0, 1000, 0, 1000); // Bounds
    w.write_rect(0, 1000, 0, 1000); // EdgeBounds

    // 1 byte flags: reserved=0, winding=1, non_scaling=0, scaling=0
    w.write_ub(0, 5);
    w.write_ub(1, 1); // uses non-zero winding rule
    w.write_ub(0, 1);
    w.write_ub(0, 1);
    w.align_to_byte();

    // FillStyleArray: 1 focal gradient
    w.write_ui8(1);
    w.write_ui8(0x13); // FocalRadialGradient
    w.write_matrix_identity();
    w.write_ub(0, 2); // pad
    w.write_ub(0, 2); // normal rgb
    w.write_ub(2, 4); // 2 stops
    w.write_ui8(0); // ratio 0
    w.write_ui8(255);
    w.write_ui8(255);
    w.write_ui8(255);
    w.write_ui8(255);
    w.write_ui8(255); // ratio 255
    w.write_ui8(0);
    w.write_ui8(0);
    w.write_ui8(0);
    w.write_ui8(255);
    w.write_sb(0, 16); // focal point 0.0

    // LineStyleArray: 0
    w.write_ui8(0);

    w.write_ub(1, 4);
    w.write_ub(0, 4);

    // StyleChange: MoveTo (0, 0), fill1 = 1
    w.write_ub(0, 1);
    w.write_ub(0b00101, 5);
    w.write_ub(11, 5);
    w.write_sb(0, 11);
    w.write_sb(0, 11);
    w.write_ub(1, 1);

    // StraightEdge: (0,0) -> (1000, 0)
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(10, 4);
    w.write_ub(0, 1);
    w.write_ub(0, 1);
    w.write_sb(1000, 12);

    // StraightEdge: (1000,0) -> (1000, 1000)
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(10, 4);
    w.write_ub(0, 1);
    w.write_ub(1, 1);
    w.write_sb(1000, 12);

    // StraightEdge: (1000,1000) -> (0, 0)
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(10, 4);
    w.write_ub(1, 1);
    w.write_sb(-1000, 12);
    w.write_sb(-1000, 12);

    // EndShapeRecord
    w.write_ub(0, 1);
    w.write_ub(0, 5);

    let bytes = w.finish();
    let parsed = parse_define_shape(83, &bytes).expect("parse DefineShape4");
    assert!(parsed.uses_non_zero_winding);
    assert_eq!(
        parsed.edge_bounds,
        Some(SwfRect::create_rect(0, 1000, 0, 1000))
    );

    let scene = lower_shape_to_paint_scene(&parsed);
    // 2 instructions: 1 Gradient definition, 1 Path referencing url(#gradient)
    assert_eq!(scene.instructions.len(), 2);

    match &scene.instructions[0] {
        PaintInstruction::Gradient(g) => {
            assert!(matches!(g.kind, GradientKind::Radial { .. }));
            assert_eq!(g.stops.len(), 2);
        }
        _ => panic!("expected Gradient instruction"),
    }

    match &scene.instructions[1] {
        PaintInstruction::Path(p) => {
            assert!(p.fill.as_ref().unwrap().starts_with("url(#shape_7_fill_0)"));
            assert_eq!(p.fill_rule, Some(FillRule::NonZero));
        }
        _ => panic!("expected Path instruction"),
    }
}

#[test]
fn test_define_shape2_parsing_and_new_styles() {
    let mut w = BitWriter::new();
    w.write_ui16_le(10); // ShapeId
    w.write_rect(0, 500, 0, 500);

    // Initial FillStyleArray: 1 solid color
    w.write_ui8(1);
    w.write_ui8(0x00); // Solid
    w.write_ui8(100);
    w.write_ui8(150);
    w.write_ui8(200);

    // Initial LineStyleArray: 0
    w.write_ui8(0);

    w.write_ub(1, 4);
    w.write_ub(0, 4);

    // StyleChange: MoveTo(0,0), fill1 = 1
    w.write_ub(0, 1);
    w.write_ub(0b00101, 5);
    w.write_ub(9, 5);
    w.write_sb(0, 9);
    w.write_sb(0, 9);
    w.write_ub(1, 1);

    // Edge 1: (0,0) -> (200, 0)
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(8, 4);
    w.write_ub(0, 1);
    w.write_ub(0, 1);
    w.write_sb(200, 10);

    // StyleChange with StateNewStyles: define new fill style
    w.write_ub(0, 1);
    w.write_ub(0b10000, 5); // state_new_styles (bit 4)
    w.align_to_byte();

    // New FillStyleArray: 1 solid green
    w.write_ui8(1);
    w.write_ui8(0x00);
    w.write_ui8(0);
    w.write_ui8(255);
    w.write_ui8(0);

    // New LineStyleArray: 0
    w.write_ui8(0);

    // New num_fill_bits = 1, num_line_bits = 0
    w.write_ub(1, 4);
    w.write_ub(0, 4);

    // StyleChange: select fill1 = 1 from new style table, MoveTo (200, 0)
    w.write_ub(0, 1);
    w.write_ub(0b00101, 5); // state_fill_style1 | state_move_to
    w.write_ub(9, 5);
    w.write_sb(200, 9);
    w.write_sb(0, 9);
    w.write_ub(1, 1); // fill1 = 1 (in new array, so global index 2)

    // Edge 2: (200,0) -> (200, 200)
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(8, 4);
    w.write_ub(0, 1);
    w.write_ub(1, 1);
    w.write_sb(200, 10);

    // EndShapeRecord
    w.write_ub(0, 1);
    w.write_ub(0, 5);

    let bytes = w.finish();
    let parsed = parse_define_shape(22, &bytes).expect("parse DefineShape2 with StateNewStyles");
    assert_eq!(parsed.parsed_fill_style_count(), 2);
    assert_eq!(parsed.parsed_edge_count(), 2);

    let scene = lower_shape_to_paint_scene(&parsed);
    // 2 separate fill paths (one for each fill style)
    assert_eq!(scene.instructions.len(), 2);
}

#[test]
fn test_bitmap_fill_lowering() {
    let mut w = BitWriter::new();
    w.write_ui16_le(5);
    w.write_rect(0, 100, 0, 100);

    // 1 Clipped bitmap fill
    w.write_ui8(1);
    w.write_ui8(0x41); // ClippedBitmap
    w.write_ui16_le(101); // Bitmap ID
    w.write_matrix_identity();

    w.write_ui8(0); // Line styles
    w.write_ub(1, 4);
    w.write_ub(0, 4);

    // MoveTo(0,0), fill1 = 1
    w.write_ub(0, 1);
    w.write_ub(0b00101, 5);
    w.write_ub(7, 5);
    w.write_sb(0, 7);
    w.write_sb(0, 7);
    w.write_ub(1, 1);

    // Edge (0,0) -> (100, 100)
    w.write_ub(1, 1);
    w.write_ub(1, 1);
    w.write_ub(6, 4); // nbits = 8
    w.write_ub(1, 1); // general line
    w.write_sb(100, 8);
    w.write_sb(100, 8);

    w.write_ub(0, 1);
    w.write_ub(0, 5);

    let bytes = w.finish();
    let parsed = parse_define_shape(32, &bytes).expect("parse bitmap fill");
    let scene = lower_shape_to_paint_scene(&parsed);
    assert_eq!(scene.instructions.len(), 1);

    match &scene.instructions[0] {
        PaintInstruction::Path(path) => {
            assert_eq!(path.fill.as_deref(), Some("swf-bitmap://101"));
        }
        _ => panic!("expected Path with bitmap url"),
    }
}

#[test]
fn test_security_bounds_and_caps() {
    // 1. Tag code validation
    assert!(parse_define_shape(1, &[0; 10]).is_err());
    assert!(parse_define_shape(99, &[0; 10]).is_err());

    // 2. Truncated EOF
    assert!(parse_define_shape(2, &[]).is_err());
    assert!(parse_define_shape(2, &[0, 1]).is_err());

    // 3. Overflow coordinate guard
    assert_eq!(
        checked_twips_add(MAX_SAFE_TWIPS, 10),
        Err(ShapeError::ArithmeticOverflow)
    );
    assert_eq!(
        checked_twips_add(MIN_SAFE_TWIPS, -10),
        Err(ShapeError::ArithmeticOverflow)
    );
}
