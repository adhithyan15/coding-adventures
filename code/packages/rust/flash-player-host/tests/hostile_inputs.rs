//! Hostile-input security regression and fuzz tests for `flash-player-host` (`SWF07`).
//!
//! Covers:
//! - Hostile URIs (control bytes, WHATWG `/\`, multibyte UTF-8, `> 2048` bytes, `fscommand:exec`/`save`)
//! - Hostile `WindowEvent`s (`NaN` / `±Inf` / `1e100` / subnormals)
//! - Atomic `(clock, stage, audio)` rollback on `PatchBufferTooSmall`, `HierarchyDepthExceeded`,
//!   and `ScriptTimeout` (with `audit_log` preserved!)
//! - Atomic `player_step_clock_and_audio_zero_alloc` rollback on `PatchBufferTooSmall`
//! - `player_execute_source_script` blocking and auditing `__proto__` / `prototype` / `constructor` (CWE-1321)
//! - Canonical `NaN` payload & `-0.0` normalization in `compute_scene_digest_hex`
//! - Seeded 1,500-case hostile SWF / URI / `WindowEvent` fuzz campaign

#![forbid(unsafe_code)]

use flash_display_list::{DisplayListError, DisplayObject, DisplaySlot, MovieClip};
use flash_player_host::{
    compute_scene_digest_hex, sanitize_navigation_uri, AsDialect, FlashPlayer, FlashPlayerConfig,
    LogicalSize, PaintInstruction, PaintScene, PatchKind, PatchOp, PlayerHostError,
    SecurityAuditKind, WindowEvent, WindowId, MAX_URI_LEN,
};
use paint_instructions::PaintRect;
use paint_palette_blend::PaletteBlendError;

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let v = self.next_u64().to_le_bytes();
            let take = (len - out.len()).min(8);
            out.extend_from_slice(&v[..take]);
        }
        out
    }
}

fn encode_tag(code: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if payload.len() < 0x3F {
        let word = (code << 6) | (payload.len() as u16);
        out.extend_from_slice(&word.to_le_bytes());
    } else {
        let word = (code << 6) | 0x3F;
        out.extend_from_slice(&word.to_le_bytes());
        out.extend_from_slice(&(payload.len() as i32).to_le_bytes());
    }
    out.extend_from_slice(payload);
    out
}

fn build_swf_fws(version: u8, frame_rate_fps: u8, frame_count: u16, body: &[u8]) -> Vec<u8> {
    // 15-bit RECT: 0..11000 twips x 0..8000 twips (550x400 px)
    let mut rect_bits = Vec::new();
    let nbits = 15u32;
    let vals = [0i32, 11_000, 0, 8_000];
    let mut bit_buf: u64 = 0;
    let mut bit_len: u32 = 0;
    let push_bits = |val: u32, count: u32, buf: &mut u64, len: &mut u32, out: &mut Vec<u8>| {
        *buf = (*buf << count) | (val as u64 & ((1u64 << count) - 1));
        *len += count;
        while *len >= 8 {
            *len -= 8;
            out.push(((*buf >> *len) & 0xFF) as u8);
        }
    };
    push_bits(nbits, 5, &mut bit_buf, &mut bit_len, &mut rect_bits);
    for v in vals {
        push_bits(v as u32, nbits, &mut bit_buf, &mut bit_len, &mut rect_bits);
    }
    if bit_len > 0 {
        rect_bits.push(((bit_buf << (8 - bit_len)) & 0xFF) as u8);
    }

    let file_len = 8 + rect_bits.len() + 4 + body.len();
    let mut swf = Vec::with_capacity(file_len);
    swf.extend_from_slice(b"FWS");
    swf.push(version);
    swf.extend_from_slice(&(file_len as u32).to_le_bytes());
    swf.extend_from_slice(&rect_bits);
    swf.extend_from_slice(&[0u8, frame_rate_fps]);
    swf.extend_from_slice(&frame_count.to_le_bytes());
    swf.extend_from_slice(body);
    swf
}

fn build_define_shape_square(shape_id: u16, r: u8, g: u8, b: u8) -> Vec<u8> {
    struct BitWriter {
        bytes: Vec<u8>,
        cur: u8,
        used: u8,
    }
    impl BitWriter {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                cur: 0,
                used: 0,
            }
        }
        fn write_ub(&mut self, val: u32, bits: u8) {
            for i in (0..bits).rev() {
                let bit = ((val >> i) & 1) as u8;
                self.cur = (self.cur << 1) | bit;
                self.used += 1;
                if self.used == 8 {
                    self.bytes.push(self.cur);
                    self.cur = 0;
                    self.used = 0;
                }
            }
        }
        fn write_sb(&mut self, val: i32, bits: u8) {
            self.write_ub(val as u32, bits);
        }
        fn align(&mut self) {
            if self.used > 0 {
                self.cur <<= 8 - self.used;
                self.bytes.push(self.cur);
                self.cur = 0;
                self.used = 0;
            }
        }
    }

    let mut payload = Vec::new();
    payload.extend_from_slice(&shape_id.to_le_bytes());

    let mut bw = BitWriter::new();
    bw.write_ub(10, 5);
    bw.write_sb(0, 10);
    bw.write_sb(400, 10);
    bw.write_sb(0, 10);
    bw.write_sb(400, 10);
    bw.align();
    payload.extend_from_slice(&bw.bytes);

    payload.push(1);
    payload.push(0x00);
    payload.extend_from_slice(&[r, g, b]);
    payload.push(0);

    let mut sbw = BitWriter::new();
    sbw.write_ub(1, 4);
    sbw.write_ub(0, 4);
    sbw.write_ub(0, 1);
    sbw.write_ub(0, 1);
    sbw.write_ub(0, 1);
    sbw.write_ub(1, 1);
    sbw.write_ub(0, 1);
    sbw.write_ub(0, 1);
    sbw.write_ub(1, 1);

    let write_edge = |w: &mut BitWriter, dx: i32, dy: i32| {
        w.write_ub(1, 1);
        w.write_ub(1, 1);
        w.write_ub(8, 4);
        w.write_ub(1, 1);
        w.write_sb(dx, 10);
        w.write_sb(dy, 10);
    };
    write_edge(&mut sbw, 400, 0);
    write_edge(&mut sbw, 0, 400);
    write_edge(&mut sbw, -400, 0);
    write_edge(&mut sbw, 0, -400);

    sbw.write_ub(0, 6);
    sbw.align();
    payload.extend_from_slice(&sbw.bytes);

    encode_tag(2, &payload)
}

fn build_place_object2_new(depth: u16, char_id: u16) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(0x02);
    payload.extend_from_slice(&depth.to_le_bytes());
    payload.extend_from_slice(&char_id.to_le_bytes());
    encode_tag(26, &payload)
}

fn build_minimal_multiframe_swf(frames: u16) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&encode_tag(9, &[0x11, 0x22, 0x33])); // SetBackgroundColor
    body.extend_from_slice(&build_define_shape_square(1, 0xFF, 0x00, 0x00));
    body.extend_from_slice(&build_place_object2_new(1, 1));
    for _ in 0..frames {
        body.extend_from_slice(&encode_tag(1, &[])); // ShowFrame
    }
    body.extend_from_slice(&encode_tag(0, &[])); // End
    build_swf_fws(9, 24, frames, &body)
}

fn build_infinite_loop_avm1_swf() -> Vec<u8> {
    // DoAction (Tag 12): ActionBranchAlways(-5) -> tight infinite loop in ActionScript VM
    let mut actions = Vec::new();
    actions.push(0x99);
    actions.extend_from_slice(&2u16.to_le_bytes());
    actions.extend_from_slice(&(-5i16).to_le_bytes());
    actions.push(0x00); // ActionEnd

    let mut body = Vec::new();
    body.extend_from_slice(&encode_tag(12, &actions));
    body.extend_from_slice(&encode_tag(1, &[])); // ShowFrame
    body.extend_from_slice(&encode_tag(0, &[])); // End
    build_swf_fws(8, 24, 1, &body)
}

#[test]
fn hostile_uris_and_fscommand_neutralization() {
    let bad_uris = [
        "",
        "   \t\r\n  ",
        "java\tscript:alert(1)",
        "java\x00script:alert(1)",
        "javascript:alert(1)",
        "JAVASCRIPT:alert(1)",
        "vbscript:msgbox(1)",
        "data:text/html,<script>alert(1)</script>",
        "file:///etc/passwd",
        "//evil.example/phish",
        "/\\evil.example/phish",
        "\\\\server\\share\\file",
        "https://evil.example/path\\back",
        "https://café.example/\u{0007}bell",
    ];

    for uri in &bad_uris {
        assert!(
            sanitize_navigation_uri(uri).is_err(),
            "hostile URI must be rejected: {uri:?}"
        );
    }

    let oversized = format!("https://example.com/{}", "a".repeat(MAX_URI_LEN));
    assert_eq!(
        sanitize_navigation_uri(&oversized),
        Err(PlayerHostError::InvalidUri)
    );
    // Multibyte UTF-8 crossing MAX_URI_LEN byte boundary
    let multibyte = format!("https://example.com/{}", "é".repeat(MAX_URI_LEN));
    assert_eq!(
        sanitize_navigation_uri(&multibyte),
        Err(PlayerHostError::InvalidUri)
    );

    let swf = build_minimal_multiframe_swf(1);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    assert_eq!(
        player.player_navigate_url("fscommand:exec", "calc.exe"),
        Err(PlayerHostError::BlockedFsCommand)
    );
    assert_eq!(
        player.player_navigate_url("  FSCOMMAND:save\x00 ", "out.txt"),
        Err(PlayerHostError::BlockedFsCommand)
    );
    assert_eq!(
        player.player_navigate_url("javascript:alert(1)", "_self"),
        Err(PlayerHostError::BlockedUriScheme)
    );
    assert!(player
        .player_audit_events()
        .iter()
        .any(|e| e.audit_kind() == SecurityAuditKind::FsCommandExecBlocked));
    assert!(player
        .player_audit_events()
        .iter()
        .any(|e| e.audit_kind() == SecurityAuditKind::UriBlocked));
}

#[test]
fn hostile_window_events_rejected_without_panic() {
    let swf = build_minimal_multiframe_swf(1);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    let wid = WindowId(1);

    let bad_pointers = [
        (f64::NAN, 10.0),
        (10.0, f64::from_bits(0x7ff8_0000_0000_0001)),
        (f64::INFINITY, 0.0),
        (0.0, f64::NEG_INFINITY),
        (1e100, 0.0),
        (0.0, -1e100),
    ];
    for &(x, y) in &bad_pointers {
        assert_eq!(
            player.player_dispatch_window_event(&WindowEvent::PointerMoved {
                window_id: wid,
                x,
                y,
            }),
            Err(PlayerHostError::NonFiniteCoordinate)
        );
    }

    // Subnormal pointer coordinates are finite and must succeed safely without panicking.
    assert!(player
        .player_dispatch_window_event(&WindowEvent::PointerMoved {
            window_id: wid,
            x: f64::from_bits(1),
            y: -0.0,
        })
        .is_ok());

    let bad_sizes = [
        LogicalSize::new(f64::NAN, 100.0),
        LogicalSize::new(100.0, f64::INFINITY),
        LogicalSize::new(0.0, 100.0),
        LogicalSize::new(1e100, 100.0),
    ];
    for size in bad_sizes {
        assert_eq!(
            player.player_dispatch_window_event(&WindowEvent::Resized {
                window_id: wid,
                logical_size: size,
                physical_size: window_core::PhysicalSize::new(100, 100),
                scale_factor: 1.0,
            }),
            Err(PlayerHostError::InvalidStageDimension)
        );
    }
}

#[test]
fn hostile_atomic_step_rollback_preserves_state_and_audit_log() {
    // 1. PatchBufferTooSmall in player_step_with_clock must roll back clock, stage, and audio.
    let swf = build_minimal_multiframe_swf(4);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    let clock_before = *player.player_clock();
    let frame_before = player.player_timeline_frame();
    let scene_before = player.player_last_scene().clone();

    let mut empty_patches: Vec<PatchOp> = Vec::new();
    let err = player
        .player_step_with_clock(50_000_000, &mut empty_patches, None)
        .expect_err("empty patch buffer must fail with PatchBufferTooSmall");
    assert!(matches!(
        err,
        PlayerHostError::Blend(PaletteBlendError::PatchBufferTooSmall { .. })
    ));
    assert_eq!(*player.player_clock(), clock_before);
    assert_eq!(player.player_timeline_frame(), frame_before);
    assert_eq!(player.player_last_scene(), &scene_before);

    // 2. PatchBufferTooSmall in player_step_clock_and_audio_zero_alloc must also leave clock/stage untouched.
    let mut next_scene = scene_before.clone();
    let mut rect = PaintRect::filled(0.0, 0.0, 10.0, 10.0, "#ff0000");
    rect.base.id = Some("hostile-rect-1".to_string());
    next_scene.instructions.push(PaintInstruction::Rect(rect));
    let mut audio_buf = [0i16; 64];
    let err_za = player
        .player_step_clock_and_audio_zero_alloc(
            50_000_000,
            &scene_before,
            &next_scene,
            &mut empty_patches,
            &mut audio_buf,
        )
        .expect_err("zero_alloc step must fail cleanly on empty patch buffer");
    assert!(matches!(
        err_za,
        PlayerHostError::Blend(PaletteBlendError::PatchBufferTooSmall { .. })
    ));
    assert_eq!(*player.player_clock(), clock_before);
    assert_eq!(player.player_timeline_frame(), frame_before);

    // 3. HierarchyDepthExceeded in player_step_with_clock rolls back clock & timeline frame.
    let mut deep_clip = MovieClip::new(1);
    for _ in 0..70 {
        let mut parent = MovieClip::new(1);
        parent
            .clip_insert_slot(DisplaySlot::new(1, DisplayObject::Clip(deep_clip)))
            .unwrap();
        deep_clip = parent;
    }
    player
        .player_stage_mut()
        .root
        .clip_insert_slot(DisplaySlot::new(99, DisplayObject::Clip(deep_clip)))
        .unwrap();
    let clock_before_depth = *player.player_clock();
    let frame_before_depth = player.player_timeline_frame();
    let mut patches = vec![
        PatchOp {
            kind: PatchKind::Added,
            old_index: 0,
            new_index: 0,
        };
        64
    ];
    let err_depth = player
        .player_step_with_clock(50_000_000, &mut patches, None)
        .expect_err("deep hierarchy must fail");
    assert_eq!(
        err_depth,
        PlayerHostError::DisplayList(DisplayListError::HierarchyDepthExceeded)
    );
    assert_eq!(*player.player_clock(), clock_before_depth);
    assert_eq!(player.player_timeline_frame(), frame_before_depth);

    // 4. ScriptTimeout must roll back clock/stage/audio WHILE preserving `audit_log`.
    let loop_swf = build_infinite_loop_avm1_swf();
    let mut cfg = FlashPlayerConfig::cfg_standard();
    cfg.max_vm_steps_per_frame = 200;
    let mut loop_player = FlashPlayer::player_from_swf_bytes(&loop_swf, cfg).unwrap();
    let loop_clock_before = *loop_player.player_clock();
    let loop_frame_before = loop_player.player_timeline_frame();
    let mut loop_patches = vec![
        PatchOp {
            kind: PatchKind::Added,
            old_index: 0,
            new_index: 0,
        };
        64
    ];

    let err_timeout = loop_player
        .player_step_with_clock(50_000_000, &mut loop_patches, None)
        .expect_err("infinite loop SWF must hit ScriptTimeout");
    assert_eq!(err_timeout, PlayerHostError::ScriptTimeout);
    assert_eq!(*loop_player.player_clock(), loop_clock_before);
    assert_eq!(loop_player.player_timeline_frame(), loop_frame_before);
    assert!(
        loop_player
            .player_audit_events()
            .iter()
            .any(|e| e.audit_kind() == SecurityAuditKind::ScriptTimeout),
        "ScriptTimeout audit event must be preserved across atomic rollback"
    );
}

#[test]
fn hostile_execute_source_script_blocks_and_audits_cwe1321() {
    let swf = build_minimal_multiframe_swf(1);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();

    let payloads = [
        "var o = {}; o.__proto__ = 1;",
        "var o = {}; trace(o.__proto__);",
        "var o = {}; o.__proto__();",
        "var o = {}; o.prototype = 2;",
        "var o = {}; trace(o.constructor);",
    ];

    for (idx, src) in payloads.iter().enumerate() {
        let res = player.player_execute_source_script(src, AsDialect::As3);
        assert!(
            matches!(
                res,
                Err(PlayerHostError::Lifter(
                    avm_bytecode_lifter::LifterError::PrototypePollutionBlocked(_)
                ))
            ),
            "expected PrototypePollutionBlocked for `{src}`, got {res:?}"
        );
        let pp_count = player
            .player_audit_events()
            .iter()
            .filter(|e| e.audit_kind() == SecurityAuditKind::PrototypePollutionBlocked)
            .count();
        assert_eq!(pp_count, idx + 1);
    }
}

#[test]
fn hostile_canonical_nan_and_negative_zero_scene_digest() {
    let mut s1 = PaintScene::new(100.0, 100.0);
    s1.width = f64::NAN;
    s1.height = -0.0;

    let mut s2 = PaintScene::new(100.0, 100.0);
    s2.width = f64::from_bits(0x7ff8_0000_0000_0001);
    s2.height = 0.0;

    let mut s3 = PaintScene::new(100.0, 100.0);
    s3.width = f64::from_bits(0xfff8_abcd_1234_5678);
    s3.height = -0.0;

    let d1 = compute_scene_digest_hex(&s1);
    let d2 = compute_scene_digest_hex(&s2);
    let d3 = compute_scene_digest_hex(&s3);
    assert_eq!(d1, d2);
    assert_eq!(d1, d3);
}

#[test]
fn hostile_fuzz_1500_cases_swf_uri_events_zero_panics() {
    let mut rng = SplitMix64(0x5AF7_0007_2026_1500);
    let valid_swf = build_minimal_multiframe_swf(2);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&valid_swf, FlashPlayerConfig::cfg_standard()).unwrap();
    let wid = WindowId(0);

    for i in 0..1_500 {
        let len = (rng.next_u64() as usize) % 256;
        let bytes = rng.next_bytes(len);
        let _ = FlashPlayer::player_from_swf_bytes(&bytes, FlashPlayerConfig::cfg_standard());

        let lossy = String::from_utf8_lossy(&bytes);
        let _ = sanitize_navigation_uri(&lossy);

        let fx = f64::from_bits(rng.next_u64());
        let fy = f64::from_bits(rng.next_u64());
        let ev = match i % 3 {
            0 => WindowEvent::PointerMoved {
                window_id: wid,
                x: fx,
                y: fy,
            },
            1 => WindowEvent::Resized {
                window_id: wid,
                logical_size: LogicalSize::new(fx, fy),
                physical_size: window_core::PhysicalSize::new(100, 100),
                scale_factor: 1.0,
            },
            _ => WindowEvent::RedrawRequested { window_id: wid },
        };
        let _ = player.player_dispatch_window_event(&ev);
    }
}
