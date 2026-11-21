//! Comprehensive conformance test suite for `flash-player-host` (`SWF07`).

#![forbid(unsafe_code)]

use std::cell::{Cell, RefCell};

use flash_display_list::PlayState;
use flash_player_host::{
    compute_scene_digest_hex, evaluate_fscommand, is_fscommand_exec, run_headless_swf,
    sanitize_navigation_uri, validate_frame_rate_hz, validate_stage_dimension_px,
    validate_uri_length, validate_vm_step_budget, verify_headless_digests, AsDialect, ElementState,
    FlashHostSession, FlashPlayer, FlashPlayerConfig, FsCommandOutcome, HeadlessRunConfig, Key,
    LogicalSize, ModifiersState, NamedKey, PaintAcceleration, PaintBackendCapabilities,
    PaintBackendDescriptor, PaintBackendFamily, PaintBackendRegistry, PaintBackendTier,
    PaintPlatformSupport, PaintRenderError, PaintRenderOptions, PaintRenderer, PaintScene,
    PatchKind, PatchOp, PlayerHostError, PointerButton, SecurityAuditKind, SupportLevel, Window,
    WindowAttributes, WindowBackend, WindowError, WindowEvent, WindowId, MAX_HOST_FRAME_RATE_HZ,
    MAX_HOST_VM_STEPS, MAX_STAGE_DIM_PX, MAX_URI_LEN,
};
use paint_instructions::PixelContainer;

// ---------------------------------------------------------------------------
// Clean-room synthetic SWF binary builder for tests
// ---------------------------------------------------------------------------

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
    // Simple 1-byte RECT: nbits=1 (0b00001_0_0_0_0 = 0x08 -> 5+4=9 bits = 2 bytes)
    // Better: nbits=15 signed -> 0..11000 twips (550 px) x 0..8000 twips (400 px)
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
    // 8.8 fixed point frame rate (little endian: frac=0, int=frame_rate_fps)
    swf.extend_from_slice(&[0u8, frame_rate_fps]);
    swf.extend_from_slice(&frame_count.to_le_bytes());
    swf.extend_from_slice(body);
    swf
}

fn build_define_shape_square(shape_id: u16, r: u8, g: u8, b: u8) -> Vec<u8> {
    // DefineShape (Tag 2): shape_id + RECT(0, 400, 0, 400) + FillStyleCount(1, solid RGB) + LineStyleCount(0) + NumFillBits=1, NumLineBits=0 + records + EndShapeRecord
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
    // Bounds RECT: nbits=10, 0, 400, 0, 400
    bw.write_ub(10, 5);
    bw.write_sb(0, 10);
    bw.write_sb(400, 10);
    bw.write_sb(0, 10);
    bw.write_sb(400, 10);
    bw.align();
    payload.extend_from_slice(&bw.bytes);

    // FillStyleArray: count=1, type=0x00 (solid), RGB
    payload.push(1);
    payload.push(0x00);
    payload.extend_from_slice(&[r, g, b]);
    // LineStyleArray: count=0
    payload.push(0);

    // ShapeWithStyle: NumFillBits=1 (4 bits), NumLineBits=0 (4 bits) -> 0x10
    let mut sbw = BitWriter::new();
    sbw.write_ub(1, 4);
    sbw.write_ub(0, 4);
    // StyleChangeRecord: TypeFlag=0, StateNewStyles=0, StateLineStyle=0, StateFillStyle1=1, StateFillStyle0=0, StateMoveTo=0 -> 0_0_0_1_0_0
    sbw.write_ub(0, 1);
    sbw.write_ub(0, 1);
    sbw.write_ub(0, 1);
    sbw.write_ub(1, 1);
    sbw.write_ub(0, 1);
    sbw.write_ub(0, 1);
    sbw.write_ub(1, 1); // fill_style1 = 1

    // 4 straight edges forming a 400x400 twips (20x20 px) square:
    // nbits - 2 = 8 -> nbits = 10
    let write_edge = |w: &mut BitWriter, dx: i32, dy: i32| {
        w.write_ub(1, 1); // TypeFlag = 1 (edge)
        w.write_ub(1, 1); // StraightFlag = 1
        w.write_ub(8, 4); // NumBits - 2 = 8 => 10 bits
        w.write_ub(1, 1); // GeneralLineFlag = 1
        w.write_sb(dx, 10);
        w.write_sb(dy, 10);
    };
    write_edge(&mut sbw, 400, 0);
    write_edge(&mut sbw, 0, 400);
    write_edge(&mut sbw, -400, 0);
    write_edge(&mut sbw, 0, -400);

    // EndShapeRecord: 6 zero bits
    sbw.write_ub(0, 6);
    sbw.align();
    payload.extend_from_slice(&sbw.bytes);

    encode_tag(2, &payload)
}

fn build_place_object2_new(depth: u16, char_id: u16) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(0x02); // HasCharacter
    payload.extend_from_slice(&depth.to_le_bytes());
    payload.extend_from_slice(&char_id.to_le_bytes());
    encode_tag(26, &payload)
}

fn build_place_object2_move_translate(depth: u16, tx_twips: i32, ty_twips: i32) -> Vec<u8> {
    // Flags: HasMove (0x01) | HasMatrix (0x04) = 0x05
    let mut payload = Vec::new();
    payload.push(0x05);
    payload.extend_from_slice(&depth.to_le_bytes());
    // MATRIX with HasScale=0, HasRotate=0, NTranslateBits=15
    let mut bit_buf: u64 = 0;
    let mut bit_len: u32 = 0;
    let mut m_bytes = Vec::new();
    let push_bits = |val: u32, count: u32, buf: &mut u64, len: &mut u32, out: &mut Vec<u8>| {
        *buf = (*buf << count) | (val as u64 & ((1u64 << count) - 1));
        *len += count;
        while *len >= 8 {
            *len -= 8;
            out.push(((*buf >> *len) & 0xFF) as u8);
        }
    };
    push_bits(0, 1, &mut bit_buf, &mut bit_len, &mut m_bytes); // HasScale=0
    push_bits(0, 1, &mut bit_buf, &mut bit_len, &mut m_bytes); // HasRotate=0
    push_bits(15, 5, &mut bit_buf, &mut bit_len, &mut m_bytes); // NTranslateBits=15
    push_bits(
        tx_twips as u32,
        15,
        &mut bit_buf,
        &mut bit_len,
        &mut m_bytes,
    );
    push_bits(
        ty_twips as u32,
        15,
        &mut bit_buf,
        &mut bit_len,
        &mut m_bytes,
    );
    if bit_len > 0 {
        m_bytes.push(((bit_buf << (8 - bit_len)) & 0xFF) as u8);
    }
    payload.extend_from_slice(&m_bytes);
    encode_tag(26, &payload)
}

fn build_remove_object2(depth: u16) -> Vec<u8> {
    encode_tag(28, &depth.to_le_bytes())
}

fn build_set_bg_color(r: u8, g: u8, b: u8) -> Vec<u8> {
    encode_tag(9, &[r, g, b])
}

fn build_define_sound_pcm16(sound_id: u16, samples: &[i16]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&sound_id.to_le_bytes());
    // Format=3 (UncompressedLe, <<4 = 0x30) | Rate=3 (44100, <<2 = 0x0C) | Size=1 (16-bit, <<1 = 0x02) | Mono=0 => 0x3E
    payload.push(0x3E);
    payload.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    for &s in samples {
        payload.extend_from_slice(&s.to_le_bytes());
    }
    encode_tag(14, &payload)
}

fn build_start_sound(sound_id: u16) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&sound_id.to_le_bytes());
    payload.push(0x00); // SoundInfo flags = 0 (play once)
    encode_tag(15, &payload)
}

// ---------------------------------------------------------------------------
// Mock UI11 WindowBackend & P2D01 PaintRenderer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct MockWindow {
    id: WindowId,
    logical_size: LogicalSize,
    render_target: window_core::RenderTarget,
}

impl Window for MockWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn logical_size(&self) -> LogicalSize {
        self.logical_size
    }
    fn physical_size(&self) -> window_core::PhysicalSize {
        window_core::PhysicalSize::new(
            self.logical_size.width as u32,
            self.logical_size.height as u32,
        )
    }
    fn scale_factor(&self) -> f64 {
        1.0
    }
    fn request_redraw(&self) -> Result<(), WindowError> {
        Ok(())
    }
    fn set_title(&self, _title: &str) -> Result<(), WindowError> {
        Ok(())
    }
    fn set_visible(&self, _visible: bool) -> Result<(), WindowError> {
        Ok(())
    }
    fn render_target(&self) -> window_core::RenderTarget {
        self.render_target.clone()
    }
}

struct MockWindowBackend {
    queued_events: RefCell<Vec<WindowEvent>>,
    created_count: Cell<u64>,
}

impl MockWindowBackend {
    fn with_events(events: Vec<WindowEvent>) -> Self {
        Self {
            queued_events: RefCell::new(events),
            created_count: Cell::new(0),
        }
    }
}

impl WindowBackend for MockWindowBackend {
    type Window = MockWindow;

    fn backend_name(&self) -> &'static str {
        "headless-mock"
    }

    fn create_window(&mut self, attributes: WindowAttributes) -> Result<Self::Window, WindowError> {
        let next = self.created_count.get() + 1;
        self.created_count.set(next);
        Ok(MockWindow {
            id: WindowId(next),
            logical_size: attributes.initial_size,
            render_target: window_core::RenderTarget::Win32(window_core::Win32RenderTarget {
                hwnd: 0x1234,
            }),
        })
    }

    fn pump_events(&mut self) -> Result<Vec<WindowEvent>, WindowError> {
        Ok(std::mem::take(&mut *self.queued_events.borrow_mut()))
    }
}

struct MockPaintRenderer;

impl PaintRenderer for MockPaintRenderer {
    fn descriptor(&self) -> PaintBackendDescriptor {
        PaintBackendDescriptor {
            id: "mock-software",
            display_name: "Mock Software",
            family: PaintBackendFamily::Cairo,
            acceleration: PaintAcceleration::Software,
            tier: PaintBackendTier::Tier2NativeScenes,
            platforms: PaintPlatformSupport::gpu_portable(),
            capabilities: PaintBackendCapabilities {
                rect: SupportLevel::Supported,
                path: SupportLevel::Supported,
                group: SupportLevel::Supported,
                group_transform: SupportLevel::Supported,
                group_opacity: SupportLevel::Supported,
                offscreen_pixels: SupportLevel::Supported,
                ..PaintBackendCapabilities::none()
            },
            priority: 1,
        }
    }

    fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        Ok(PixelContainer::new(
            scene.width.max(1.0) as u32,
            scene.height.max(1.0) as u32,
        ))
    }
}

// ---------------------------------------------------------------------------
// Conformance Tests
// ---------------------------------------------------------------------------

#[test]
fn test_warrant_bounds_validation() {
    assert_eq!(validate_vm_step_budget(0), Ok(true));
    assert_eq!(validate_vm_step_budget(MAX_HOST_VM_STEPS), Ok(true));
    assert_eq!(
        validate_vm_step_budget(MAX_HOST_VM_STEPS + 1),
        Err(PlayerHostError::InvalidStepBudget)
    );

    assert_eq!(validate_frame_rate_hz(1), Ok(true));
    assert_eq!(validate_frame_rate_hz(60), Ok(true));
    assert_eq!(validate_frame_rate_hz(MAX_HOST_FRAME_RATE_HZ), Ok(true));
    assert_eq!(
        validate_frame_rate_hz(0),
        Err(PlayerHostError::InvalidFrameRate)
    );
    assert_eq!(
        validate_frame_rate_hz(MAX_HOST_FRAME_RATE_HZ + 1),
        Err(PlayerHostError::InvalidFrameRate)
    );

    assert_eq!(validate_stage_dimension_px(1), Ok(true));
    assert_eq!(validate_stage_dimension_px(MAX_STAGE_DIM_PX), Ok(true));
    assert_eq!(
        validate_stage_dimension_px(0),
        Err(PlayerHostError::InvalidStageDimension)
    );
    assert_eq!(
        validate_stage_dimension_px(MAX_STAGE_DIM_PX + 1),
        Err(PlayerHostError::InvalidStageDimension)
    );

    assert_eq!(validate_uri_length(1), Ok(true));
    assert_eq!(validate_uri_length(MAX_URI_LEN), Ok(true));
    assert_eq!(validate_uri_length(0), Err(PlayerHostError::InvalidUri));
    assert_eq!(
        validate_uri_length(MAX_URI_LEN + 1),
        Err(PlayerHostError::InvalidUri)
    );
}

#[test]
fn test_load_and_step_swf_shapes_timeline_and_diff() {
    let mut body = Vec::new();
    body.extend_from_slice(&build_set_bg_color(0x10, 0x20, 0x30));
    body.extend_from_slice(&build_define_shape_square(1, 0xFF, 0x00, 0x00));
    // Frame 1: place shape 1 at depth 1
    body.extend_from_slice(&build_place_object2_new(1, 1));
    body.extend_from_slice(&encode_tag(1, &[])); // ShowFrame
                                                 // Frame 2: translate shape at depth 1 by (40 px, 20 px)
    body.extend_from_slice(&build_place_object2_move_translate(1, 800, 400));
    body.extend_from_slice(&encode_tag(1, &[])); // ShowFrame
                                                 // Frame 3: remove shape at depth 1
    body.extend_from_slice(&build_remove_object2(1));
    body.extend_from_slice(&encode_tag(1, &[])); // ShowFrame
    body.extend_from_slice(&encode_tag(0, &[])); // End

    let swf = build_swf_fws(8, 24, 3, &body);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();

    assert_eq!(player.player_shape_count(), 1);
    assert_eq!(player.player_total_frames(), 3);
    assert_eq!(player.player_frame_rate_fps(), 24.0);

    let dt_ns = player.player_clock().fixed_dt_ns;
    let mut ops = vec![
        PatchOp {
            kind: PatchKind::Added,
            old_index: 0,
            new_index: 0,
        };
        64
    ];

    // Step 1 -> executes Frame 1, advances timeline to 2
    let t1 = player
        .player_step_with_clock(dt_ns, &mut ops, None)
        .unwrap();
    assert_eq!(t1.steps_run, 1);
    assert_eq!(t1.timeline_frame, 2);
    assert_eq!(player.player_stage().stage_background_color(), "#102030");
    assert!(t1.patch_summary.added >= 1);
    let digest_f1 = player.player_scene_digest_hex();

    // Step 2 -> executes Frame 2 (moved), advances timeline to 3
    let t2 = player
        .player_step_with_clock(dt_ns, &mut ops, None)
        .unwrap();
    assert_eq!(t2.timeline_frame, 3);
    assert!(t2.patch_summary.modified >= 1);
    let digest_f2 = player.player_scene_digest_hex();
    assert_ne!(digest_f1, digest_f2);

    // Step 3 -> executes Frame 3 (removed), wraps timeline to 1
    let t3 = player
        .player_step_with_clock(dt_ns, &mut ops, None)
        .unwrap();
    assert_eq!(t3.timeline_frame, 1);
    assert!(t3.patch_summary.removed >= 1);
}

#[test]
fn test_avm1_doaction_trace_and_timeline_stop() {
    // Build ActionPush("hello flash") + ActionTrace + ActionStop + End
    let mut actions = Vec::new();
    // ActionPush (0x96): len = 1 + 11 + 1 = 13
    let msg = b"hello flash\0";
    actions.push(0x96);
    actions.extend_from_slice(&((1 + msg.len()) as u16).to_le_bytes());
    actions.push(0x00); // type 0 = string
    actions.extend_from_slice(msg);
    actions.push(0x26); // ActionTrace
    actions.push(0x07); // ActionStop
    actions.push(0x00); // ActionEnd

    let mut body = Vec::new();
    body.extend_from_slice(&encode_tag(12, &actions)); // DoAction
    body.extend_from_slice(&encode_tag(1, &[])); // ShowFrame
    body.extend_from_slice(&encode_tag(1, &[])); // ShowFrame (frame 2)
    body.extend_from_slice(&encode_tag(0, &[]));

    let swf = build_swf_fws(8, 24, 2, &body);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    let dt_ns = player.player_clock().fixed_dt_ns;
    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 16];

    let _ = player
        .player_step_with_clock(dt_ns, &mut ops, None)
        .unwrap();
    assert!(player.player_trace_output().contains("hello flash"));
    assert_eq!(
        player.player_stage().root.clip_play_state(),
        PlayState::Stop
    );
    // Because clip_stop() ran before stage_advance_frame(), timeline remains at frame 1
    assert_eq!(player.player_timeline_frame(), 1);
}

#[test]
fn test_avm1_script_timeout_cwe400() {
    // ActionBranchAlways (0x99) with offset -5 (infinite loop back to itself)
    let mut actions = Vec::new();
    actions.push(0x99);
    actions.extend_from_slice(&2u16.to_le_bytes());
    actions.extend_from_slice(&(-5i16).to_le_bytes());
    actions.push(0x00);

    let mut body = Vec::new();
    body.extend_from_slice(&encode_tag(12, &actions));
    body.extend_from_slice(&encode_tag(1, &[]));
    body.extend_from_slice(&encode_tag(0, &[]));

    let swf = build_swf_fws(8, 24, 1, &body);
    let mut cfg = FlashPlayerConfig::cfg_standard();
    cfg.max_vm_steps_per_frame = 200;

    let mut player = FlashPlayer::player_from_swf_bytes(&swf, cfg).unwrap();
    let dt_ns = player.player_clock().fixed_dt_ns;
    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 8];

    let err = player
        .player_step_with_clock(dt_ns, &mut ops, None)
        .unwrap_err();
    assert_eq!(err, PlayerHostError::ScriptTimeout);
    assert!(player
        .player_audit_events()
        .iter()
        .any(|e| e.kind == SecurityAuditKind::ScriptTimeout));
}

#[test]
fn test_avm2_doabc_execution() {
    // Minimal valid ABC file (minor=16, major=46) returning void
    let mut abc = Vec::new();
    abc.extend_from_slice(&16u16.to_le_bytes());
    abc.extend_from_slice(&46u16.to_le_bytes());
    // Constant pool: int=0, uint=0, double=0, string=0, ns=0, nsset=0, multiname=0
    abc.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0]);
    // Methods=1: param_count=0, return_type=0, name=0, flags=0
    abc.extend_from_slice(&[1, 0, 0, 0, 0]);
    // Metadata=0, Classes=0, Scripts=1 (init=0, traits=0)
    abc.extend_from_slice(&[0, 0, 1, 0, 0]);
    // MethodBodies=1: method=0, max_stack=1, local_count=1, init_scope=0, max_scope=1, code_len=1 (returnvoid=0x47), exceptions=0, traits=0
    abc.extend_from_slice(&[1, 0, 1, 1, 0, 1, 1, 0x47, 0, 0]);

    // Wrap with SWF9 DoABC2 header: flags(4) + "main\0" + abc
    let mut doabc_payload = Vec::new();
    doabc_payload.extend_from_slice(&1u32.to_le_bytes());
    doabc_payload.extend_from_slice(b"main\0");
    doabc_payload.extend_from_slice(&abc);

    let mut body = Vec::new();
    body.extend_from_slice(&encode_tag(82, &doabc_payload));
    body.extend_from_slice(&encode_tag(1, &[]));
    body.extend_from_slice(&encode_tag(0, &[]));

    let swf = build_swf_fws(9, 24, 1, &body);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    let dt_ns = player.player_clock().fixed_dt_ns;
    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 8];

    let res = player.player_step_with_clock(dt_ns, &mut ops, None);
    assert!(res.is_ok());
}

#[test]
fn test_audio_define_sound_and_start_sound_sync() {
    let samples = [1200i16; 256];
    let mut body = Vec::new();
    body.extend_from_slice(&build_define_sound_pcm16(1, &samples));
    body.extend_from_slice(&build_start_sound(1));
    body.extend_from_slice(&encode_tag(1, &[]));
    body.extend_from_slice(&encode_tag(0, &[]));

    let swf = build_swf_fws(8, 24, 1, &body);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    let dt_ns = player.player_clock().fixed_dt_ns;
    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 8];
    let mut pcm_out = [0i16; 128];

    let tick = player
        .player_step_with_clock(dt_ns, &mut ops, Some(&mut pcm_out))
        .unwrap();
    assert_eq!(tick.audio_samples_rendered, 128);
    assert!(pcm_out.iter().any(|&s| s != 0));
}

#[test]
fn test_ui11_window_input_and_paint_vm_runtime_session() {
    let mut body = Vec::new();
    body.extend_from_slice(&build_define_shape_square(1, 0x00, 0xAA, 0xFF));
    body.extend_from_slice(&build_place_object2_new(1, 1));
    body.extend_from_slice(&encode_tag(1, &[]));
    body.extend_from_slice(&encode_tag(0, &[]));

    let swf = build_swf_fws(8, 24, 1, &body);
    let player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();

    let events = vec![
        WindowEvent::PointerMoved {
            window_id: WindowId(1),
            x: 120.0,
            y: 80.0,
        },
        WindowEvent::PointerButton {
            window_id: WindowId(1),
            button: PointerButton::Primary,
            state: ElementState::Pressed,
        },
        WindowEvent::Key {
            window_id: WindowId(1),
            key: Key::Named(NamedKey::Space),
            state: ElementState::Pressed,
            modifiers: ModifiersState::default(),
            text: Some(" ".to_string()),
        },
        WindowEvent::CloseRequested {
            window_id: WindowId(1),
        },
    ];
    let backend = MockWindowBackend::with_events(events);
    let mut session = FlashHostSession::session_create(player, backend, "SWF07 Session").unwrap();

    let dt_ns = session.session_player().player_clock().fixed_dt_ns;
    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 32];

    let _tick = session
        .session_pump_and_step(dt_ns, &mut ops, None)
        .unwrap();

    assert_eq!(
        session
            .session_player()
            .player_stage()
            .stage_mouse_position(),
        (120.0, 80.0)
    );
    assert!(session
        .session_player()
        .player_stage()
        .stage_is_mouse_down());
    assert!(session
        .session_player()
        .player_keyboard()
        .kbd_is_named_down(&NamedKey::Space));
    assert!(session.session_player().player_close_requested());

    let renderer = MockPaintRenderer;
    let mut registry = PaintBackendRegistry::new();
    registry.register(&renderer);
    let pixels = session
        .session_render(&registry, PaintRenderOptions::default())
        .unwrap();
    assert_eq!((pixels.width, pixels.height), (550, 400));
}

#[test]
fn test_headless_ci_runner_and_sha256_digests() {
    let mut body = Vec::new();
    body.extend_from_slice(&build_define_shape_square(1, 0x22, 0x88, 0x44));
    body.extend_from_slice(&build_place_object2_new(1, 1));
    body.extend_from_slice(&encode_tag(1, &[]));
    body.extend_from_slice(&build_place_object2_move_translate(1, 200, 100));
    body.extend_from_slice(&encode_tag(1, &[]));
    body.extend_from_slice(&encode_tag(0, &[]));

    let swf = build_swf_fws(8, 24, 2, &body);
    let player_cfg = FlashPlayerConfig::cfg_standard();
    let mut run_cfg = HeadlessRunConfig::headless_cfg_for_frames(2);
    run_cfg.render_audio_per_frame = true;

    let report_a = run_headless_swf(&swf, &player_cfg, &run_cfg).unwrap();
    let report_b = run_headless_swf(&swf, &player_cfg, &run_cfg).unwrap();

    assert_eq!(report_a.frames.len(), 2);
    assert_eq!(
        report_a.report_frame_digest(1),
        report_b.report_frame_digest(1)
    );
    assert_eq!(
        report_a.report_frame_digest(2),
        report_b.report_frame_digest(2)
    );
    assert_ne!(
        report_a.report_frame_digest(1),
        report_a.report_frame_digest(2)
    );
    assert!(report_a.frames[0].audio_digest_hex.is_some());

    let expected = vec![
        (1u32, report_a.report_frame_digest(1).unwrap().to_string()),
        (2u32, report_a.report_frame_digest(2).unwrap().to_string()),
    ];
    assert!(verify_headless_digests(&report_a, &expected).is_ok());

    let bad_expected = vec![(1u32, "0".repeat(64))];
    assert!(matches!(
        verify_headless_digests(&report_a, &bad_expected),
        Err(PlayerHostError::DigestMismatch { frame: 1, .. })
    ));

    // Also verify canonical PaintScene hashing ignores HashMap key order
    let mut s1 = PaintScene::new(100.0, 100.0);
    let mut s2 = PaintScene::new(100.0, 100.0);
    s1.metadata = Some(std::collections::HashMap::from([
        ("b".to_string(), "2".to_string()),
        ("a".to_string(), "1".to_string()),
    ]));
    s2.metadata = Some(std::collections::HashMap::from([
        ("a".to_string(), "1".to_string()),
        ("b".to_string(), "2".to_string()),
    ]));
    assert_eq!(compute_scene_digest_hex(&s1), compute_scene_digest_hex(&s2));
}

#[test]
fn test_cwe78_fscommand_exec_permanently_disabled() {
    assert!(is_fscommand_exec("exec"));
    assert!(is_fscommand_exec("  \tExEc\n "));
    assert!(!is_fscommand_exec("fullscreen"));

    assert_eq!(
        evaluate_fscommand("exec", "cmd.exe /c dir"),
        FsCommandOutcome::BlockedNoOp
    );
    assert_eq!(
        evaluate_fscommand("allowscale", "false"),
        FsCommandOutcome::HandledNoOp
    );

    let swf = build_swf_fws(8, 24, 1, &encode_tag(0, &[]));
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();

    assert_eq!(
        player.player_dispatch_fscommand("  EXEC\x00 ", "/bin/sh"),
        FsCommandOutcome::BlockedNoOp
    );
    assert_eq!(
        player.player_navigate_url("fscommand:exec", "calc.exe"),
        Err(PlayerHostError::BlockedFsCommand)
    );
    assert_eq!(
        player
            .player_audit_events()
            .iter()
            .filter(|e| e.kind == SecurityAuditKind::FsCommandExecBlocked)
            .count(),
        2
    );
}

#[test]
fn test_cwe94_uri_scheme_sanitization() {
    // Allowed URIs
    assert_eq!(
        sanitize_navigation_uri("http://example.com/path"),
        Ok("http://example.com/path")
    );
    assert_eq!(
        sanitize_navigation_uri("  \n\tHTTPS://Example.COM/a?x=1#y  "),
        Ok("HTTPS://Example.COM/a?x=1#y")
    );
    assert_eq!(
        sanitize_navigation_uri("/assets/level1.swf"),
        Ok("/assets/level1.swf")
    );
    assert_eq!(
        sanitize_navigation_uri("sub/page.html?k=v#h"),
        Ok("sub/page.html?k=v#h")
    );

    // Blocked dangerous schemes (CWE-94 / CWE-79)
    for bad in [
        "javascript:alert(1)",
        "  JaVaScRiPt:alert(1)  ",
        "\x01javascript:alert(1)\x00",
        "file:///etc/passwd",
        "FILE:///C:/Windows/win.ini",
        "data:text/html,<script>alert(1)</script>",
        "DATA:application/octet-stream;base64,AA==",
        "vbscript:msgbox(1)",
        "ftp://example.com/a",
        "//evil.example/spoof",
    ] {
        assert_eq!(
            sanitize_navigation_uri(bad),
            Err(PlayerHostError::BlockedUriScheme),
            "expected BlockedUriScheme for {bad:?}"
        );
    }

    // Embedded control characters / invalid URIs
    assert_eq!(
        sanitize_navigation_uri("java\tscript:alert(1)"),
        Err(PlayerHostError::InvalidUri)
    );
    assert_eq!(
        sanitize_navigation_uri("   \t\n "),
        Err(PlayerHostError::InvalidUri)
    );
    assert_eq!(
        sanitize_navigation_uri("\\\\server\\share"),
        Err(PlayerHostError::BlockedUriScheme)
    );

    // Player navigateToURL audit recording
    let swf = build_swf_fws(8, 24, 1, &encode_tag(0, &[]));
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    assert_eq!(
        player.player_navigate_url("https://safe.example.org/play", "_blank"),
        Ok("https://safe.example.org/play")
    );
    assert_eq!(
        player.player_navigate_url("javascript:evil()", "_self"),
        Err(PlayerHostError::BlockedUriScheme)
    );
    assert_eq!(player.player_navigation_requests().len(), 1);
    assert!(player
        .player_audit_events()
        .iter()
        .any(|e| e.kind == SecurityAuditKind::UriBlocked));
}

#[test]
fn test_execute_inline_actionscript_source_and_anti_bomb() {
    let swf = build_swf_fws(8, 24, 1, &encode_tag(0, &[]));
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    player
        .player_execute_source_script("trace(\"inline ok\");", AsDialect::As2)
        .unwrap();
    assert!(player.player_trace_output().contains("inline ok"));

    // Invalid signature rejected cleanly
    assert!(
        FlashPlayer::player_from_swf_bytes(b"BAD!", FlashPlayerConfig::cfg_standard()).is_err()
    );
}

#[test]
fn test_define_sprite_and_sound_stream_sync() {
    let mut body = Vec::new();
    body.extend_from_slice(&build_define_shape_square(1, 0x33, 0x66, 0x99));

    // DefineSprite (Tag 39): sprite_id=2, frame_count=2, PlaceObject2(depth=1, char_id=1), ShowFrame, End
    let mut sprite_payload = Vec::new();
    sprite_payload.extend_from_slice(&2u16.to_le_bytes());
    sprite_payload.extend_from_slice(&2u16.to_le_bytes());
    sprite_payload.extend_from_slice(&build_place_object2_new(1, 1));
    sprite_payload.extend_from_slice(&encode_tag(1, &[]));
    sprite_payload.extend_from_slice(&encode_tag(0, &[]));
    body.extend_from_slice(&encode_tag(39, &sprite_payload));

    // SoundStreamHead2 (Tag 45): Playback=0x38 (44100/16bit/Mono), Stream=0x3E (UncompressedLe), AvgSamples=16
    let mut head_payload = Vec::new();
    head_payload.push(0x38);
    head_payload.push(0x3E);
    head_payload.extend_from_slice(&16u16.to_le_bytes());
    body.extend_from_slice(&encode_tag(45, &head_payload));

    // Frame 1: PlaceObject2 sprite 2 at depth 1 + SoundStreamBlock (16 PCM16 samples) + ShowFrame
    body.extend_from_slice(&build_place_object2_new(1, 2));
    let mut block_payload = Vec::new();
    for i in 1..=16i16 {
        block_payload.extend_from_slice(&(i * 250).to_le_bytes());
    }
    body.extend_from_slice(&encode_tag(19, &block_payload));
    body.extend_from_slice(&encode_tag(1, &[]));
    body.extend_from_slice(&encode_tag(0, &[]));

    let swf = build_swf_fws(8, 24, 1, &body);
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    assert_eq!(player.player_sprite_count(), 1);

    let dt_ns = player.player_clock().fixed_dt_ns;
    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 32];
    let mut pcm = [0i16; 16];
    let tick = player
        .player_step_with_clock(dt_ns, &mut ops, Some(&mut pcm))
        .unwrap();
    assert_eq!(tick.audio_samples_rendered, 16);
    assert!(pcm.iter().any(|&s| s != 0));
    assert!(!player.player_last_scene().instructions.is_empty());
}

#[test]
fn test_ui11_resize_focus_and_non_finite_coords() {
    let swf = build_swf_fws(8, 24, 1, &encode_tag(0, &[]));
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();

    // Key press then focus lost clears pressed key
    player
        .player_dispatch_window_event(&WindowEvent::Key {
            window_id: WindowId(1),
            key: Key::Character("a".to_string()),
            state: ElementState::Pressed,
            modifiers: ModifiersState {
                shift: true,
                ..ModifiersState::default()
            },
            text: Some("a".to_string()),
        })
        .unwrap();
    assert!(player.player_keyboard().kbd_is_char_down("a"));
    assert_eq!(player.player_keyboard().kbd_last_key_code(), b'A' as u32);

    player
        .player_dispatch_window_event(&WindowEvent::FocusChanged {
            window_id: WindowId(1),
            focused: false,
        })
        .unwrap();
    assert!(!player.player_keyboard().kbd_is_char_down("a"));

    // Resize updates stage dimensions
    player
        .player_dispatch_window_event(&WindowEvent::Resized {
            window_id: WindowId(1),
            logical_size: LogicalSize::new(640.0, 480.0),
            physical_size: window_core::PhysicalSize::new(640, 480),
            scale_factor: 1.0,
        })
        .unwrap();
    assert_eq!(player.player_stage().stage_dimensions(), (640.0, 480.0));

    // Non-finite pointer coordinates rejected (CWE-20)
    let err = player
        .player_dispatch_window_event(&WindowEvent::PointerMoved {
            window_id: WindowId(1),
            x: f64::NAN,
            y: 10.0,
        })
        .unwrap_err();
    assert_eq!(err, PlayerHostError::NonFiniteCoordinate);
}

#[test]
fn test_spiral_of_death_clock_clamp() {
    let swf = build_swf_fws(8, 60, 4, &encode_tag(0, &[]));
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).unwrap();
    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 16];

    // 1 second delta at 60 Hz exceeds 250ms delta cap & 8-step clamp
    let tick = player
        .player_step_with_clock(1_000_000_000, &mut ops, None)
        .unwrap();
    assert!(tick.steps_run <= 8);
    assert!(tick.spiral_of_death_clamped || tick.dropped_ns > 0);
}
