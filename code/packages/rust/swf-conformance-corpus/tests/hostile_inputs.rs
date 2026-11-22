//! Hostile-input security regression and fuzz tests for `swf-conformance-corpus` (`SWF08`).
//!
//! Covers:
//! - `NaN` / `±Inf` / `1e20` `SwfMatrix` and 16.16 fixed-point `fb` values
//! - Embedded NUL (`\0`) rejection across `Avm1ActionBuilder` and tag builders
//! - `SwfBitWriter` atomic rollback on `Err` at `MAX_CORPUS_SWF_BYTES` and mid-record overflows
//! - `Avm1ActionBuilder` atomic `MAX_CORPUS_SWF_BYTES` cap
//! - Malformed SHA-256 digest hex string rejection (`validate_corpus_digest_hex`)
//! - Hostile `FlashPlayerConfig` rejection (`run_headless_swf` / `FlashPlayer::player_from_swf_bytes`)
//! - Seeded 1,000-case `SwfBitWriter` + `SwfBuilder` + `Avm1ActionBuilder` fuzz campaign

#![forbid(unsafe_code)]

use swf_conformance_corpus::{
    encode_do_abc_tag, encode_place_object_tag, encode_swf_tag, min_signed_bit_width,
    min_unsigned_bit_width, validate_corpus_bit_width, validate_corpus_digest_hex,
    validate_corpus_frame_count, validate_corpus_swf_bytes, Avm1ActionBuilder, CorpusError,
    FlashPlayer, FlashPlayerConfig, PlaceTagSpec, SwfBitWriter, SwfBuilder, SwfCompression,
    MAX_BIT_WIDTH, MAX_CORPUS_FRAMES, MAX_CORPUS_SWF_BYTES,
};
use swf_container::{SwfColorTransform, SwfMatrix};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_hostile_f64(&mut self) -> f64 {
        match self.next_u64() % 10 {
            0 => f64::NAN,
            1 => f64::from_bits(0x7ff8_0000_0000_0001),
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => -0.0,
            5 => 1e20,
            6 => -1e20,
            7 => f64::MIN_POSITIVE * 0.5,
            8 => f64::MAX,
            _ => {
                let u = ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64);
                (u * 2.0 - 1.0) * 100.0
            }
        }
    }
}

#[test]
fn hostile_swf_matrix_and_fb_nan_inf_overflow() {
    let hostile_floats = [
        f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e20,
        -1e20,
    ];

    for &v in &hostile_floats {
        let mut bw = SwfBitWriter::bit_writer_new();
        bw.bit_writer_write_ub(0b101, 3).unwrap();
        let len_before = bw.bit_writer_byte_len();
        assert_eq!(
            bw.bit_writer_write_fb(v, 16),
            Err(CorpusError::IntegerOverflow)
        );
        assert_eq!(bw.bit_writer_byte_len(), len_before);
    }

    let hostile_matrices = [
        SwfMatrix {
            scale_x: f64::NAN,
            scale_y: 1.0,
            rotate_skew_0: 0.0,
            rotate_skew_1: 0.0,
            translate_x: 0,
            translate_y: 0,
        },
        SwfMatrix {
            scale_x: 1e20,
            scale_y: 1.0,
            rotate_skew_0: 0.0,
            rotate_skew_1: 0.0,
            translate_x: 0,
            translate_y: 0,
        },
        SwfMatrix {
            scale_x: 2.0,
            scale_y: 2.0,
            rotate_skew_0: 1e20,
            rotate_skew_1: 0.0,
            translate_x: 100,
            translate_y: 200,
        },
        SwfMatrix {
            scale_x: 2.0,
            scale_y: 2.0,
            rotate_skew_0: 0.5,
            rotate_skew_1: 0.5,
            translate_x: i32::MIN,
            translate_y: 0,
        },
    ];

    for m in &hostile_matrices {
        let mut bw = SwfBitWriter::bit_writer_new();
        bw.bit_writer_write_ub(0b10101, 5).unwrap();
        let err = bw
            .bit_writer_write_matrix(m)
            .expect_err("hostile SwfMatrix must fail");
        assert!(matches!(
            err,
            CorpusError::IntegerOverflow | CorpusError::BitWidthExceeded
        ));
        // Atomic rollback: only the original 5 bits remain.
        let bytes = bw.bit_writer_into_bytes().unwrap();
        assert_eq!(bytes, vec![0b1010_1000]);
    }
}

#[test]
fn hostile_embedded_nul_and_size_cap_in_builders() {
    let mut ab = Avm1ActionBuilder::avm1_new();
    ab.avm1_push_i32(42);
    let baseline = ab.clone().avm1_finish();

    assert!(matches!(
        ab.avm1_push_str("ok\0smuggled"),
        Err(CorpusError::UnexpectedCaseFailure(_))
    ));
    assert!(matches!(
        ab.avm1_define_function("fn\0name", &["a"], &[]),
        Err(CorpusError::UnexpectedCaseFailure(_))
    ));
    assert!(matches!(
        ab.avm1_define_function("fn_ok", &["a\0b"], &[]),
        Err(CorpusError::UnexpectedCaseFailure(_))
    ));
    assert!(matches!(
        ab.avm1_define_function2("fn2\0", 2, 0, &[(1, "p")], &[]),
        Err(CorpusError::UnexpectedCaseFailure(_))
    ));
    assert!(matches!(
        ab.avm1_define_function2("fn2", 2, 0, &[(1, "p\0q")], &[]),
        Err(CorpusError::UnexpectedCaseFailure(_))
    ));
    assert_eq!(ab.clone().avm1_finish(), baseline);

    // Atomic MAX_CORPUS_SWF_BYTES cap in Avm1ActionBuilder.
    let big_body = vec![0x01u8; 60_000];
    let mut cap_ab = Avm1ActionBuilder::avm1_new();
    for _ in 0..279 {
        cap_ab.avm1_define_function("f", &[], &big_body).unwrap();
    }
    let before_bytes = cap_ab.clone().avm1_finish();
    assert_eq!(
        cap_ab.avm1_define_function("overflow", &[], &big_body),
        Err(CorpusError::SwfSizeExceeded)
    );
    assert_eq!(cap_ab.avm1_finish(), before_bytes);

    // DoABC and PlaceObject NUL guards.
    assert!(matches!(
        encode_do_abc_tag("abc\0bad", &[]),
        Err(CorpusError::UnexpectedCaseFailure(_))
    ));
    let mut place = PlaceTagSpec::place_new(1, 1);
    place.name = Some("clip\0bad".to_string());
    assert!(matches!(
        encode_place_object_tag(&place),
        Err(CorpusError::UnexpectedCaseFailure(_))
    ));
}

#[test]
fn hostile_bit_writer_atomic_rollback_at_max_corpus_swf_bytes() {
    let mut bw = SwfBitWriter::bit_writer_new();
    // Fill up to MAX_CORPUS_SWF_BYTES - 2 bytes.
    let fill_chunk = vec![0xABu8; 1_048_576];
    for _ in 0..15 {
        bw.bit_writer_write_bytes(&fill_chunk).unwrap();
    }
    let rem = MAX_CORPUS_SWF_BYTES - 15 * 1_048_576 - 2;
    bw.bit_writer_write_bytes(&vec![0xCDu8; rem]).unwrap();
    assert_eq!(bw.bit_writer_byte_len(), MAX_CORPUS_SWF_BYTES - 2);

    // Writing 24 bits (3 bytes) must fail atomically with SwfSizeExceeded without changing byte_len.
    assert_eq!(
        bw.bit_writer_write_ub(0x12_3456, 24),
        Err(CorpusError::SwfSizeExceeded)
    );
    assert_eq!(bw.bit_writer_byte_len(), MAX_CORPUS_SWF_BYTES - 2);

    // Writing a RECT that exceeds remaining 2 bytes must roll back atomically.
    assert_eq!(
        bw.bit_writer_write_rect(-1000, 1000, -1000, 1000),
        Err(CorpusError::SwfSizeExceeded)
    );
    assert_eq!(bw.bit_writer_byte_len(), MAX_CORPUS_SWF_BYTES - 2);

    // Writing a CXFORM that exceeds remaining 2 bytes must roll back atomically.
    let cx = SwfColorTransform {
        r_mult: 300,
        g_mult: 256,
        b_mult: 256,
        a_mult: 256,
        r_add: 100,
        g_add: 0,
        b_add: 0,
        a_add: 0,
    };
    assert_eq!(
        bw.bit_writer_write_cxform_alpha(&cx),
        Err(CorpusError::SwfSizeExceeded)
    );
    assert_eq!(bw.bit_writer_byte_len(), MAX_CORPUS_SWF_BYTES - 2);

    let final_bytes = bw.bit_writer_into_bytes().unwrap();
    assert_eq!(final_bytes.len(), MAX_CORPUS_SWF_BYTES - 2);
}

#[test]
fn hostile_digest_hex_and_player_config_limits() {
    assert_eq!(
        validate_corpus_digest_hex(b""),
        Err(CorpusError::InvalidDigestFormat)
    );
    assert_eq!(
        validate_corpus_digest_hex(&[b'0'; 63]),
        Err(CorpusError::InvalidDigestFormat)
    );
    assert_eq!(
        validate_corpus_digest_hex(&[b'0'; 65]),
        Err(CorpusError::InvalidDigestFormat)
    );
    let mut bad_hex = [b'a'; 64];
    bad_hex[31] = b'g';
    assert_eq!(
        validate_corpus_digest_hex(&bad_hex),
        Err(CorpusError::InvalidDigestFormat)
    );
    assert_eq!(validate_corpus_digest_hex(&[b'f'; 64]), Ok(true));

    assert_eq!(
        validate_corpus_swf_bytes(0),
        Err(CorpusError::SwfSizeExceeded)
    );
    assert_eq!(
        validate_corpus_swf_bytes(MAX_CORPUS_SWF_BYTES + 1),
        Err(CorpusError::SwfSizeExceeded)
    );
    assert_eq!(
        validate_corpus_frame_count(0),
        Err(CorpusError::FrameCountExceeded)
    );
    assert_eq!(
        validate_corpus_frame_count(MAX_CORPUS_FRAMES + 1),
        Err(CorpusError::FrameCountExceeded)
    );
    assert_eq!(
        validate_corpus_bit_width((MAX_BIT_WIDTH as usize) + 1),
        Err(CorpusError::BitWidthExceeded)
    );

    // Build a minimal valid SWF and test hostile FlashPlayerConfig values.
    let mut b = SwfBuilder::swf_builder_new(9, SwfCompression::Fws, 100, 100, 24).unwrap();
    b.swf_builder_show_frame().unwrap();
    let swf = b.swf_builder_finish().unwrap();

    let mut cfg_zero_steps = FlashPlayerConfig::cfg_standard();
    cfg_zero_steps.max_vm_steps_per_frame = 0;
    assert!(FlashPlayer::player_from_swf_bytes(&swf, cfg_zero_steps).is_err());

    let mut cfg_huge_steps = FlashPlayerConfig::cfg_standard();
    cfg_huge_steps.max_vm_steps_per_frame = usize::MAX;
    assert!(FlashPlayer::player_from_swf_bytes(&swf, cfg_huge_steps).is_err());

    let mut cfg_zero_tick = FlashPlayerConfig::cfg_standard();
    cfg_zero_tick.max_frame_steps_per_tick = 0;
    assert!(FlashPlayer::player_from_swf_bytes(&swf, cfg_zero_tick).is_err());

    let mut cfg_nan_ratio = FlashPlayerConfig::cfg_standard();
    cfg_nan_ratio.max_expansion_ratio = f64::NAN;
    assert!(FlashPlayer::player_from_swf_bytes(&swf, cfg_nan_ratio).is_err());

    let mut cfg_neg_ratio = FlashPlayerConfig::cfg_standard();
    cfg_neg_ratio.max_expansion_ratio = -1.0;
    assert!(FlashPlayer::player_from_swf_bytes(&swf, cfg_neg_ratio).is_err());
}

#[test]
fn hostile_fuzz_1000_cases_builder_and_bitwriter_zero_panics() {
    let mut rng = SplitMix64(0x5AF8_0008_2026_1000);

    for i in 0..1_000 {
        let bits = (rng.next_u64() % 40) as u8;
        let val_u = rng.next_u64() as u32;
        let val_i = rng.next_u64() as i32;
        let _ = min_signed_bit_width(&[val_i, val_i.wrapping_neg()]);
        let _ = min_unsigned_bit_width(val_u);

        let mut bw = SwfBitWriter::bit_writer_new();
        let _ = bw.bit_writer_write_ub(val_u, bits);
        let _ = bw.bit_writer_write_sb(val_i, bits);
        let f = rng.next_hostile_f64();
        let _ = bw.bit_writer_write_fb(f, bits);
        let _ = bw.bit_writer_write_rect(val_i, val_i.wrapping_add(10), -5, 5);
        let m = SwfMatrix {
            scale_x: rng.next_hostile_f64(),
            scale_y: rng.next_hostile_f64(),
            rotate_skew_0: rng.next_hostile_f64(),
            rotate_skew_1: rng.next_hostile_f64(),
            translate_x: val_i,
            translate_y: val_i.wrapping_neg(),
        };
        let _ = bw.bit_writer_write_matrix(&m);
        let cx = SwfColorTransform {
            r_mult: (rng.next_u64() as i32),
            g_mult: 256,
            b_mult: 256,
            a_mult: 256,
            r_add: (rng.next_u64() as i32),
            g_add: 0,
            b_add: 0,
            a_add: 0,
        };
        let _ = bw.bit_writer_write_cxform_alpha(&cx);
        let _ = bw.bit_writer_into_bytes();

        let _ = encode_swf_tag((rng.next_u64() & 0x7FF) as u16, &[i as u8; 8]);
        let mut ab = Avm1ActionBuilder::avm1_new();
        let s = if i % 4 == 0 { "a\0b" } else { "ok" };
        let _ = ab.avm1_push_str(s);
        let _ = ab.avm1_define_function(s, &[s], &[0x00]);
        let _ = ab.avm1_define_function2(s, 1, 0, &[(1, s)], &[0x00]);
        let _ = ab.avm1_finish();
    }
}
