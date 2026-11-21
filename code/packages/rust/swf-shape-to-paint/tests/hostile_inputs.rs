//! Hostile-input security regression and fuzz tests for `swf-shape-to-paint` (SWF02).
//!
//! Covers:
//! - CWE-400: Cumulative `MAX_STYLES` bypass via repeated `state_new_styles` records
//! - `i32::MIN`/`i32::MAX` twips arithmetic overflow & validation guards
//! - Truncated and corrupt `DefineShape` 1–4 (`2`, `22`, `32`, `83`) payloads
//! - Seeded 1,000-case malformed shape fuzz campaign (`parse_define_shape` + `lower_shape_to_paint_scene`)

#![forbid(unsafe_code)]

use swf_shape_to_paint::{
    checked_twips_add, checked_twips_point_add, lower_shape_to_paint_scene, parse_define_shape,
    validate_twips, ShapeError, MAX_SAFE_TWIPS, MAX_STYLES, MIN_SAFE_TWIPS,
};

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

#[test]
fn hostile_cumulative_style_change_bomb_rejected() {
    // DefineShape2 (tag 22): shape_id=1, bounds=empty RECT (1 byte 0x00).
    // Initial fill styles: 0xFF followed by u16 LE = 65_000 solid RGB fill styles (4 bytes each).
    let mut data = Vec::with_capacity(65_000 * 4 + 64);
    data.extend_from_slice(&1u16.to_le_bytes()); // shape_id
    data.push(0x00); // empty RECT (nbits = 0)
    data.push(0xFF);
    data.extend_from_slice(&65_000u16.to_le_bytes());
    for _ in 0..65_000 {
        data.extend_from_slice(&[0x00, 0xFF, 0x00, 0x00]); // solid fill
    }
    data.push(0x00); // initial line_styles count = 0
    data.push(0x00); // num_fill_bits=0, num_line_bits=0

    // StyleChangeRecord with state_new_styles=1:
    // type_flag(1 bit)=0, flags(5 bits)=10000 => top 6 bits = 010000_00 = 0x40
    data.push(0x40);
    // new_fill_count = 0xFF, u16 LE = 1000 => cumulative 66_000 > MAX_STYLES (65_535)
    data.push(0xFF);
    data.extend_from_slice(&1000u16.to_le_bytes());

    let err = parse_define_shape(22, &data)
        .expect_err("cumulative fill styles > MAX_STYLES must return ExcessiveStyles");
    assert_eq!(err, ShapeError::ExcessiveStyles);

    // Same cumulative check for line styles: 65_000 initial line styles + 1000 via state_new_styles.
    let mut line_data = Vec::with_capacity(65_000 * 5 + 64);
    line_data.extend_from_slice(&2u16.to_le_bytes()); // shape_id
    line_data.push(0x00); // empty RECT
    line_data.push(0x00); // initial fill_styles count = 0
    line_data.push(0xFF);
    line_data.extend_from_slice(&65_000u16.to_le_bytes());
    for _ in 0..65_000 {
        // DefineShape2 line style: u16 width + 3-byte RGB
        line_data.extend_from_slice(&[20, 0, 0, 0, 0]);
    }
    line_data.push(0x00); // num_fill_bits=0, num_line_bits=0
    line_data.push(0x40); // StyleChangeRecord state_new_styles=1
    line_data.push(0x00); // new_fill_count = 0
    line_data.push(0xFF);
    line_data.extend_from_slice(&1000u16.to_le_bytes());

    let err = parse_define_shape(22, &line_data)
        .expect_err("cumulative line styles > MAX_STYLES must return ExcessiveStyles");
    assert_eq!(err, ShapeError::ExcessiveStyles);
    assert_eq!(MAX_STYLES, 65_535);
}

#[test]
fn hostile_twips_i32_min_max_overflow_rejected() {
    assert!(validate_twips(i32::MIN).is_err());
    assert!(validate_twips(i32::MAX).is_err());
    assert!(validate_twips(MAX_SAFE_TWIPS + 1).is_err());
    assert!(validate_twips(MIN_SAFE_TWIPS - 1).is_err());

    assert_eq!(
        checked_twips_add(i32::MAX, 1),
        Err(ShapeError::ArithmeticOverflow)
    );
    assert_eq!(
        checked_twips_add(i32::MIN, -1),
        Err(ShapeError::ArithmeticOverflow)
    );
    assert_eq!(
        checked_twips_point_add(MAX_SAFE_TWIPS, 0, 1, 0),
        Err(ShapeError::ArithmeticOverflow)
    );
    assert_eq!(
        checked_twips_point_add(0, MIN_SAFE_TWIPS, 0, -1),
        Err(ShapeError::ArithmeticOverflow)
    );
}

#[test]
fn hostile_truncated_and_corrupt_define_shape_1_to_4() {
    let tags = [2u16, 22, 32, 83];
    for &tag in &tags {
        assert!(parse_define_shape(tag, &[]).is_err());
        assert!(parse_define_shape(tag, &[0x01]).is_err());
        assert!(parse_define_shape(tag, &[0x01, 0x00, 0xFF]).is_err());
        // Valid header + RECT(0), claiming 10 fill styles with empty remainder
        assert!(parse_define_shape(tag, &[0x01, 0x00, 0x00, 0x0A]).is_err());
    }
    // Invalid tag codes rejected
    assert_eq!(parse_define_shape(0, &[0; 16]), Err(ShapeError::InvalidTag));
    assert_eq!(
        parse_define_shape(999, &[0; 16]),
        Err(ShapeError::InvalidTag)
    );
}

#[test]
fn hostile_shape_fuzz_1000_cases_no_panics() {
    let mut rng = SplitMix64(0x5F02_1337_BEEF_CAFE);
    let tags = [2u16, 22, 32, 83, 1, 99];

    for i in 0..1_000 {
        let tag = tags[i % tags.len()];
        let len = rng.next_usize(128);
        let data: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        if let Ok(shape) = parse_define_shape(tag, &data) {
            let _scene = lower_shape_to_paint_scene(&shape);
        }
    }
}
