//! Conformance & adversarial security corpus tests for `swf-conformance-corpus` (`SWF08`).

#![forbid(unsafe_code)]

use flash_display_list::DisplayListEvent;
use flash_player_host::{FlashPlayer, FlashPlayerConfig, PatchSummary};
use swf_conformance_corpus::{
    encode_define_sound_pcm16_tag, encode_define_sprite_tag, encode_do_abc_tag,
    encode_place_object_tag, encode_swf_tag, run_all_corpus_cases, run_corpus_case,
    validate_corpus_bit_width, validate_corpus_digest_hex, validate_corpus_frame_count,
    validate_corpus_swf_bytes, verify_corpus_digests, AdversarialCaseId, CorpusError, CorpusId,
    CorpusRunner, PlaceTagSpec, SwfBitWriter, SwfBuilder, SwfCompression, MAX_BIT_WIDTH,
    MAX_CORPUS_FRAMES, MAX_CORPUS_SWF_BYTES,
};
use swf_container::{SwfColorTransform, SwfMatrix, SwfMovie};

#[test]
fn test_warrant_bounds_validation() {
    assert_eq!(validate_corpus_swf_bytes(1), Ok(true));
    assert_eq!(validate_corpus_swf_bytes(MAX_CORPUS_SWF_BYTES), Ok(true));
    assert_eq!(
        validate_corpus_swf_bytes(0),
        Err(CorpusError::SwfSizeExceeded)
    );
    assert_eq!(
        validate_corpus_swf_bytes(MAX_CORPUS_SWF_BYTES + 1),
        Err(CorpusError::SwfSizeExceeded)
    );

    assert_eq!(validate_corpus_frame_count(1), Ok(true));
    assert_eq!(validate_corpus_frame_count(MAX_CORPUS_FRAMES), Ok(true));
    assert_eq!(
        validate_corpus_frame_count(0),
        Err(CorpusError::FrameCountExceeded)
    );
    assert_eq!(
        validate_corpus_frame_count(MAX_CORPUS_FRAMES + 1),
        Err(CorpusError::FrameCountExceeded)
    );

    assert_eq!(validate_corpus_bit_width(0), Ok(true));
    assert_eq!(validate_corpus_bit_width(MAX_BIT_WIDTH as usize), Ok(true));
    assert_eq!(
        validate_corpus_bit_width(MAX_BIT_WIDTH as usize + 1),
        Err(CorpusError::BitWidthExceeded)
    );

    let valid_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    assert_eq!(validate_corpus_digest_hex(valid_hex.as_bytes()), Ok(true));
    assert_eq!(
        validate_corpus_digest_hex(b"short"),
        Err(CorpusError::InvalidDigestFormat)
    );
    assert_eq!(
        validate_corpus_digest_hex(
            b"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
        ),
        Err(CorpusError::InvalidDigestFormat)
    );
}

#[test]
fn test_bit_writer_bounds_and_roundtrip() {
    let mut bw = SwfBitWriter::bit_writer_new();
    assert_eq!(
        bw.bit_writer_write_ub(1, MAX_BIT_WIDTH + 1),
        Err(CorpusError::BitWidthExceeded)
    );
    assert_eq!(
        bw.bit_writer_write_sb(1, MAX_BIT_WIDTH + 1),
        Err(CorpusError::BitWidthExceeded)
    );
    assert_eq!(
        bw.bit_writer_write_sb(5, 2),
        Err(CorpusError::BitWidthExceeded)
    );

    bw.bit_writer_write_rect(-200, 8000, -100, 6000).unwrap();
    bw.bit_writer_write_matrix(&SwfMatrix {
        scale_x: 1.5,
        scale_y: 0.75,
        rotate_skew_0: 0.25,
        rotate_skew_1: -0.25,
        translate_x: 400,
        translate_y: -200,
    })
    .unwrap();
    bw.bit_writer_write_cxform_alpha(&SwfColorTransform {
        r_mult: 128,
        g_mult: 256,
        b_mult: 64,
        a_mult: 200,
        r_add: 10,
        g_add: -5,
        b_add: 0,
        a_add: 0,
    })
    .unwrap();

    let bytes = bw.bit_writer_into_bytes().unwrap();
    let mut reader = swf_container::BitReader::from_bits(&bytes);
    let rect = reader.read_rect().unwrap();
    assert_eq!(
        (rect.xmin, rect.xmax, rect.ymin, rect.ymax),
        (-200, 8000, -100, 6000)
    );
    let mat = reader.read_matrix().unwrap();
    assert!((mat.scale_x - 1.5).abs() < 1e-3);
    assert_eq!((mat.translate_x, mat.translate_y), (400, -200));
    let cx = reader.read_cxform_with_alpha().unwrap();
    assert_eq!((cx.r_mult, cx.a_mult, cx.r_add), (128, 200, 10));
}

#[test]
fn test_builder_cws_fps88_long_tag_and_place_object3() {
    let mut b = SwfBuilder::swf_builder_new(9, SwfCompression::Cws, 400, 300, 30).unwrap();
    // 24.5 FPS = (24 << 8) | 128
    b.swf_builder_set_frame_rate_8_8((24u16 << 8) | 128)
        .unwrap();
    // Long tag (>= 0x3F bytes) using Metadata tag 77
    let meta = vec![b'x'; 128];
    b.swf_builder_push_raw_tag(&encode_swf_tag(77, &meta).unwrap())
        .unwrap();
    let mut p3 = PlaceTagSpec::place_new(1, 1);
    p3.blend_mode = Some(3);
    p3.visible = Some(true);
    b.swf_builder_place_object(&p3).unwrap();
    b.swf_builder_show_frame().unwrap();
    let cws = b.swf_builder_finish().unwrap();
    assert_eq!(&cws[..3], b"CWS");

    let movie = SwfMovie::from_swf_bytes(&cws, &Default::default()).unwrap();
    assert!((movie.movie_header().header_frame_rate() - 24.5).abs() < 1e-6);
    assert!(movie
        .movie_tags()
        .any(|t| t.is_ok_and(|tag| tag.header.is_long)));
}

#[test]
fn test_corpus_01_shapes_pinned() {
    let report = run_corpus_case(CorpusId::Shapes).unwrap();
    assert!(report.digests_verified);
    assert_eq!(report.case_report_frame_count(), 1);
    assert!(report.headless_report.frames[0].instruction_count >= 3);
}

#[test]
fn test_corpus_02_timeline_pinned() {
    let report = run_corpus_case(CorpusId::Timeline).unwrap();
    assert!(report.digests_verified);
    assert_eq!(report.case_report_frame_count(), 4);
    assert!(report.patch_summaries[0].added >= 2);
    assert!(report.patch_summaries[1].modified >= 1);
    assert!(report.patch_summaries[2].removed >= 1);
    // CWS SWF produced distinct frame digests across animation steps
    let d1 = report.case_report_frame_digest(1).unwrap();
    let d2 = report.case_report_frame_digest(2).unwrap();
    let d3 = report.case_report_frame_digest(3).unwrap();
    assert_ne!(d1, d2);
    assert_ne!(d2, d3);
}

#[test]
fn test_corpus_03_buttons_pinned() {
    let report = run_corpus_case(CorpusId::Buttons).unwrap();
    assert!(report.digests_verified);
    assert_eq!(report.case_report_frame_count(), 5);
    let actions: Vec<_> = report
        .button_events
        .iter()
        .filter(|e| matches!(e, DisplayListEvent::ButtonAction { .. }))
        .collect();
    assert_eq!(actions.len(), 4);

    let d_up1 = report.case_report_frame_digest(1).unwrap();
    let d_over1 = report.case_report_frame_digest(2).unwrap();
    let d_down = report.case_report_frame_digest(3).unwrap();
    let d_over2 = report.case_report_frame_digest(4).unwrap();
    let d_up2 = report.case_report_frame_digest(5).unwrap();
    assert_ne!(d_up1, d_over1);
    assert_ne!(d_over1, d_down);
    assert_eq!(d_over1, d_over2);
    assert_eq!(d_up1, d_up2);
}

#[test]
fn test_corpus_04_avm1_math_pinned() {
    let report = run_corpus_case(CorpusId::Avm1Math).unwrap();
    assert!(report.digests_verified);
    let trace = report.case_report_trace();
    assert!(trace.contains("sum=42"));
    assert!(trace.contains("diff=18"));
    assert!(trace.contains("prod=72"));
    assert!(trace.contains("quot=6"));
    assert!(trace.contains("less=true"));
    assert!(trace.contains("eq=true"));
}

#[test]
fn test_corpus_05_avm2_oop_pinned() {
    let report = run_corpus_case(CorpusId::Avm2Oop).unwrap();
    assert!(report.digests_verified);
    assert_eq!(report.avm2_return_value, Some(42));
}

#[test]
fn test_corpus_06_streaming_audio_pinned() {
    let report = run_corpus_case(CorpusId::StreamingAudio).unwrap();
    assert!(report.digests_verified);
    assert_eq!(report.case_report_frame_count(), 3);
    let a1 = report.case_report_audio_digest(1).unwrap();
    let a2 = report.case_report_audio_digest(2).unwrap();
    let a3 = report.case_report_audio_digest(3).unwrap();
    assert_ne!(a1, a2);
    assert_ne!(a2, a3);
}

#[test]
fn test_run_all_corpus_cases_and_digest_sabotage_detection() {
    let reports = run_all_corpus_cases().unwrap();
    assert_eq!(reports.len(), 6);
    for r in &reports {
        assert!(r.digests_verified);
    }

    // Sabotage test 1: Flip a nibble in scene digest
    let mut sabotaged_entry = CorpusRunner::runner_entry(CorpusId::Shapes);
    sabotaged_entry.expected_frame_digests[0].1 = "f".repeat(64);
    assert!(matches!(
        verify_corpus_digests(&sabotaged_entry, &reports[0]),
        Err(CorpusError::DigestMismatch { frame: 1, .. })
    ));

    // Sabotage test 2: Flip a nibble in audio digest
    let mut sabotaged_audio = CorpusRunner::runner_entry(CorpusId::StreamingAudio);
    sabotaged_audio.expected_audio_digests[1].1 = "a".repeat(64);
    assert!(matches!(
        verify_corpus_digests(&sabotaged_audio, &reports[5]),
        Err(CorpusError::DigestMismatch { frame: 2, .. })
    ));

    // Sabotage test 3: Malformed hex digest rejected
    let mut bad_hex = CorpusRunner::runner_entry(CorpusId::Shapes);
    bad_hex.expected_frame_digests[0].1 = "not_hex".to_string();
    assert_eq!(
        verify_corpus_digests(&bad_hex, &reports[0]),
        Err(CorpusError::InvalidDigestFormat)
    );

    let _ = FlashPlayer::player_from_swf_bytes;
    let _ = FlashPlayerConfig::cfg_standard();
    let _ = std::mem::size_of::<PatchSummary>();
}

#[test]
fn test_adversarial_security_corpus_all_blocked() {
    let runner = CorpusRunner::runner_new();
    let outcomes = runner.runner_execute_all_adversarial().unwrap();
    assert_eq!(
        outcomes.len(),
        AdversarialCaseId::adversarial_id_all().len()
    );
    for outcome in &outcomes {
        assert!(
            outcome.blocked,
            "adversarial case {} was not blocked",
            outcome.slug
        );
    }
}

#[test]
fn test_builder_oversized_inputs_nul_names_and_nan_matrix_rejected() {
    // 1. Oversized inputs rejected before allocation (CWE-400 / CWE-190)
    let big_bytes = vec![0u8; MAX_CORPUS_SWF_BYTES];
    assert_eq!(
        encode_define_sprite_tag(1, 1, &big_bytes),
        Err(CorpusError::SwfSizeExceeded)
    );
    assert_eq!(
        encode_do_abc_tag("main", &big_bytes),
        Err(CorpusError::SwfSizeExceeded)
    );
    let big_samples = vec![0i16; (MAX_CORPUS_SWF_BYTES / 2) + 1];
    assert_eq!(
        encode_define_sound_pcm16_tag(1, &big_samples),
        Err(CorpusError::SwfSizeExceeded)
    );

    // 2. Embedded NUL bytes rejected in NUL-terminated SWF string fields
    assert!(matches!(
        encode_do_abc_tag("bad\0name", &[0x10, 0x00, 0x2E, 0x00]),
        Err(CorpusError::UnexpectedCaseFailure(_))
    ));
    let mut place_nul = PlaceTagSpec::place_new(1, 1);
    place_nul.name = Some("bad\0instance".to_string());
    assert!(matches!(
        encode_place_object_tag(&place_nul),
        Err(CorpusError::UnexpectedCaseFailure(_))
    ));

    // 3. NaN matrix fields rejected rather than silently encoded as identity (CWE-20)
    for nan_matrix in [
        SwfMatrix {
            scale_x: f64::NAN,
            ..SwfMatrix::default_matrix()
        },
        SwfMatrix {
            scale_y: f64::NAN,
            ..SwfMatrix::default_matrix()
        },
        SwfMatrix {
            rotate_skew_0: f64::NAN,
            ..SwfMatrix::default_matrix()
        },
        SwfMatrix {
            rotate_skew_1: f64::NAN,
            ..SwfMatrix::default_matrix()
        },
    ] {
        let mut bw = SwfBitWriter::bit_writer_new();
        assert_eq!(
            bw.bit_writer_write_matrix(&nan_matrix),
            Err(CorpusError::IntegerOverflow)
        );
    }
}
