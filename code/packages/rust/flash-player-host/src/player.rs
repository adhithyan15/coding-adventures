//! Core integrated Flash player shell (`FlashPlayer` & `FlashHostSession`) (`SWF07`).
//!
//! Unites:
//! - `swf-container` (`SWF01`) for container parsing and anti-bomb decompression.
//! - `swf-shape-to-paint` (`SWF02`) for `DefineShape1..4` parsing.
//! - `flash-display-list` (`SWF05`) for `Stage`, `MovieClip`, `PlaceObject*`, `RemoveObject*`, and `PaintScene` lowering.
//! - `flash-audio-bridge` (`SWF06`) for `DefineSound`, `StartSound*`, `SoundStreamHead*`, and `SoundStreamBlock`.
//! - `avm-bytecode-lifter` (`SWF04`) & `actionscript-compiler` (`SWF03`) for bounded `vm-core` script execution (`max_steps <= 2_000_000`).
//! - `game-clock` (`GAME10`) for deterministic fixed-timestep frame pacing.
//! - `window-core` (`UI11`) for window/canvas event dispatch.
//! - `paint-vm-runtime` (`P2D01`) & `paint-palette-blend` (`P2D10`) for backend selection and scene diffing.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use actionscript_compiler::{compile_actionscript, AsDialect};
use avm_bytecode_lifter::{lift_avm1, lift_avm2, AvmRunner, LifterError};
use flash_audio_bridge::{
    parse_define_sound, parse_sound_stream_block, parse_sound_stream_head, parse_swf_audio_tag,
    render_flash_audio_into, FlashAudioState, SoundInfo, StartSoundTag, SwfAudioTag,
    SwfSoundFormat, DEFAULT_OUTPUT_SAMPLE_RATE_HZ,
};
use flash_display_list::{
    diff_paint_scene_into, parse_display_list_tag, DisplayListTag, DisplayObject, DisplaySlot,
    FrameTarget, MovieClip, PaintScene, PatchOp, PatchSummary, Stage,
};
use game_clock::{
    advance_frame, new_clock, ClockStepResult, GameClockConfig, GameClockState,
    DEFAULT_MAX_FRAME_STEPS, HARD_MAX_FRAME_STEPS,
};
use interpreter_ir::module::IIRModule;
use paint_instructions::PixelContainer;
use paint_vm_runtime::{PaintBackendRegistry, PaintRenderOptions};
use swf_container::{
    SwfDecompressorConfig, SwfMovie, SwfTagIter, SwfTagKind, DEFAULT_MAX_EXPANSION_RATIO,
    DEFAULT_MAX_UNCOMPRESSED_BYTES,
};
use swf_shape_to_paint::{parse_define_shape, ParsedShape};
use window_core::{LogicalSize, WindowBackend, WindowBuilder, WindowEvent};

use crate::digest::compute_scene_digest_hex;
use crate::error::{
    PlayerHostError, MAX_FRAME_SCRIPTS, MAX_HOST_CHARACTERS, MAX_HOST_FRAME_RATE_HZ,
    MAX_HOST_VM_STEPS, MAX_STAGE_DIM_PX, MAX_TIMELINE_FRAMES,
};
use crate::input::{dispatch_window_event_to_stage, HostInputDispatchResult, KeyboardState};
use crate::security::{
    evaluate_fscommand, sanitize_navigation_uri, trim_whitespace_and_control, FsCommandOutcome,
    SecurityAuditEvent, SecurityAuditKind, SecurityAuditLog,
};

/// Maximum navigation requests retained in the sandboxed diagnostic queue (CWE-400).
const MAX_NAVIGATION_LOG: usize = 256;

/// Owned audio event queued on a timeline frame.
#[derive(Debug, Clone)]
enum FrameAudioEvent {
    StartSound(StartSoundTag),
    StartSound2 { class_name: String, info: SoundInfo },
}

/// Tags, scripts, and audio actions belonging to a single SWF timeline frame.
#[derive(Debug, Clone, Default)]
struct TimelineFrameData {
    background_color: Option<String>,
    display_ops: Vec<DisplayListTag>,
    scripts: Vec<IIRModule>,
    audio_events: Vec<FrameAudioEvent>,
    stream_blocks: Vec<Vec<u8>>,
}

/// Configuration parameters for initializing [`FlashPlayer`].
#[derive(Debug, Clone, PartialEq)]
pub struct FlashPlayerConfig {
    /// Target PCM output sample rate in Hertz (default `44_100`).
    pub output_sample_rate_hz: u32,
    /// Maximum VM instruction budget per frame script (`<= 2_000_000`).
    pub max_vm_steps_per_frame: usize,
    /// Maximum fixed simulation ticks executed per host frame (`1..=1024`).
    pub max_frame_steps_per_tick: u32,
    /// Hard uncompressed SWF payload limit in bytes (`SWF01` anti-bomb).
    pub max_uncompressed_bytes: usize,
    /// Maximum allowed decompression expansion ratio beyond 1 MB (`SWF01` anti-bomb).
    pub max_expansion_ratio: f64,
}

impl Default for FlashPlayerConfig {
    fn default() -> Self {
        Self::cfg_standard()
    }
}

impl FlashPlayerConfig {
    /// Returns the standard hardened configuration matching `SWF00` defaults.
    pub fn cfg_standard() -> Self {
        Self {
            output_sample_rate_hz: DEFAULT_OUTPUT_SAMPLE_RATE_HZ,
            max_vm_steps_per_frame: MAX_HOST_VM_STEPS,
            max_frame_steps_per_tick: DEFAULT_MAX_FRAME_STEPS,
            max_uncompressed_bytes: DEFAULT_MAX_UNCOMPRESSED_BYTES,
            max_expansion_ratio: DEFAULT_MAX_EXPANSION_RATIO,
        }
    }

    /// Validates all configuration bounds prior to loading untrusted SWF payloads.
    pub fn cfg_validate(&self) -> Result<(), PlayerHostError> {
        if self.max_vm_steps_per_frame > MAX_HOST_VM_STEPS {
            return Err(PlayerHostError::InvalidStepBudget);
        }
        if self.max_frame_steps_per_tick == 0
            || self.max_frame_steps_per_tick > HARD_MAX_FRAME_STEPS
        {
            return Err(PlayerHostError::Clock(
                game_clock::ClockError::InvalidMaxFrameSteps,
            ));
        }
        if !self.max_expansion_ratio.is_finite() || self.max_expansion_ratio <= 0.0 {
            return Err(PlayerHostError::SwfContainer(
                swf_container::SwfError::InvalidFileLength,
            ));
        }
        Ok(())
    }
}

/// Summary of work performed during a single [`FlashPlayer::player_step_with_clock`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTickSummary {
    /// Number of fixed simulation ticks executed during this host frame.
    pub steps_run: u32,
    /// Active 1-based root timeline frame index after stepping.
    pub timeline_frame: u32,
    /// Retained-mode `PaintScene` diff summary against the previous frame.
    pub patch_summary: PatchSummary,
    /// Number of mono PCM audio samples rendered into the caller's output slice.
    pub audio_samples_rendered: usize,
    /// True if `game-clock` triggered its spiral-of-death step clamp.
    pub spiral_of_death_clamped: bool,
    /// Wall-clock nanoseconds dropped by `game-clock` clamping.
    pub dropped_ns: u64,
}

/// Summary of a zero-allocation steady-state clock + audio + diff step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerZeroAllocTick {
    /// Clock step outcome from `game-clock`.
    pub clock_step: ClockStepResult,
    /// Active 1-based timeline frame after stepping.
    pub timeline_frame: u32,
    /// Retained-mode scene diff summary.
    pub patch_summary: PatchSummary,
    /// Number of mono audio samples rendered.
    pub audio_samples_rendered: usize,
}

/// Recorded navigation request that passed CWE-94 URI sanitization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationRequest {
    /// Sanitized URI (`http:`, `https:`, or relative).
    pub uri: String,
    /// Target window/frame string (e.g. `"_self"`, `"_blank"`).
    pub target: String,
    /// 1-based timeline frame when requested.
    pub frame: u32,
}

/// Integrated Flash Player session uniting container, shapes, display list, AVM, audio, clock, and host security.
pub struct FlashPlayer {
    config: FlashPlayerConfig,
    stage: Stage,
    audio: FlashAudioState,
    clock: GameClockState,
    keyboard: KeyboardState,
    avm_runner: AvmRunner,
    shapes: BTreeMap<u16, ParsedShape>,
    sprites: BTreeMap<u16, MovieClip>,
    timeline_frames: Vec<TimelineFrameData>,
    active_stream_format: Option<SwfSoundFormat>,
    last_scene: PaintScene,
    pending_initial_frame: bool,
    stage_dirty: bool,
    close_requested: bool,
    processed_action_log_len: usize,
    audit_log: SecurityAuditLog,
    navigation_log: Vec<NavigationRequest>,
    frame_rate_fps: f64,
    declared_frame_count: u16,
}

impl FlashPlayer {
    /// Loads and validates an `.swf` binary payload into a sandboxed [`FlashPlayer`] instance.
    pub fn player_from_swf_bytes(
        swf_bytes: &[u8],
        config: FlashPlayerConfig,
    ) -> Result<Self, PlayerHostError> {
        config.cfg_validate()?;

        let decomp_cfg = SwfDecompressorConfig {
            max_uncompressed_bytes: config.max_uncompressed_bytes,
            max_expansion_ratio: config.max_expansion_ratio,
        };
        let movie = SwfMovie::from_swf_bytes(swf_bytes, &decomp_cfg)?;
        let header = movie.movie_header();

        let raw_w = header.stage_rect.rect_width_pixels();
        let raw_h = header.stage_rect.rect_height_pixels();
        if !raw_w.is_finite() || !raw_h.is_finite() {
            return Err(PlayerHostError::InvalidStageDimension);
        }
        if raw_w > MAX_STAGE_DIM_PX as f64 || raw_h > MAX_STAGE_DIM_PX as f64 {
            return Err(PlayerHostError::InvalidStageDimension);
        }
        let width_px = raw_w.max(1.0);
        let height_px = raw_h.max(1.0);

        let raw_fps = if header.frame_rate_raw == 0 {
            24.0
        } else {
            header.header_frame_rate()
        };
        if !raw_fps.is_finite() || raw_fps <= 0.0 || raw_fps > MAX_HOST_FRAME_RATE_HZ as f64 {
            return Err(PlayerHostError::InvalidFrameRate);
        }
        let fps_hz = (raw_fps.round() as u32).clamp(1, MAX_HOST_FRAME_RATE_HZ as u32);

        let mut stage = Stage::new(width_px, height_px, raw_fps);
        let mut audio = FlashAudioState::create_engine(config.output_sample_rate_hz, fps_hz)?;
        let clock_cfg = GameClockConfig::from_hz(fps_hz, config.max_frame_steps_per_tick)?;
        let clock = new_clock(clock_cfg)?;

        let mut shapes: BTreeMap<u16, ParsedShape> = BTreeMap::new();
        let mut sprites: BTreeMap<u16, MovieClip> = BTreeMap::new();
        let mut timeline_frames: Vec<TimelineFrameData> = Vec::new();
        let mut current_frame = TimelineFrameData::default();
        let mut active_stream_format: Option<SwfSoundFormat> = None;
        let mut total_scripts: usize = 0;

        for tag_res in movie.movie_tags() {
            let tag = tag_res?;
            match tag.header.kind {
                SwfTagKind::End => {
                    break;
                }
                SwfTagKind::ShowFrame => {
                    if timeline_frames.len() >= MAX_TIMELINE_FRAMES {
                        return Err(PlayerHostError::MalformedTag(
                            "timeline exceeds MAX_TIMELINE_FRAMES",
                        ));
                    }
                    timeline_frames.push(current_frame);
                    current_frame = TimelineFrameData::default();
                }
                SwfTagKind::SetBackgroundColor => {
                    if tag.data.len() < 3 {
                        return Err(PlayerHostError::MalformedTag(
                            "SetBackgroundColor shorter than 3 bytes",
                        ));
                    }
                    current_frame.background_color = Some(format!(
                        "#{:02x}{:02x}{:02x}",
                        tag.data[0], tag.data[1], tag.data[2]
                    ));
                }
                SwfTagKind::DefineShape
                | SwfTagKind::DefineShape2
                | SwfTagKind::DefineShape3
                | SwfTagKind::DefineShape4 => {
                    if shapes.len().saturating_add(sprites.len()) >= MAX_HOST_CHARACTERS {
                        return Err(PlayerHostError::CharacterLimitExceeded);
                    }
                    let shape = parse_define_shape(tag.header.kind.tag_code(), tag.data)?;
                    shapes.insert(shape.shape_id, shape);
                }
                SwfTagKind::DefineSprite => {
                    if shapes.len().saturating_add(sprites.len()) >= MAX_HOST_CHARACTERS {
                        return Err(PlayerHostError::CharacterLimitExceeded);
                    }
                    let (sprite_id, clip) = parse_define_sprite_clip(tag.data, &shapes)?;
                    sprites.insert(sprite_id, clip);
                }
                SwfTagKind::DefineSound => {
                    let ds = parse_define_sound(tag.data)?;
                    audio.register_define_sound(&ds)?;
                }
                SwfTagKind::SoundStreamHead => {
                    let head = parse_sound_stream_head(tag.data, false)?;
                    active_stream_format = Some(head.stream_properties.format);
                    audio.configure_sound_stream(head)?;
                }
                SwfTagKind::SoundStreamHead2 => {
                    let head = parse_sound_stream_head(tag.data, true)?;
                    active_stream_format = Some(head.stream_properties.format);
                    audio.configure_sound_stream(head)?;
                }
                SwfTagKind::StartSound => {
                    if let Some(SwfAudioTag::StartSound(ss)) =
                        parse_swf_audio_tag(&tag, active_stream_format)?
                    {
                        current_frame
                            .audio_events
                            .push(FrameAudioEvent::StartSound(ss));
                    }
                }
                SwfTagKind::Unknown(flash_audio_bridge::SWF_TAG_START_SOUND2) => {
                    if let Some(SwfAudioTag::StartSound2(ss2)) =
                        parse_swf_audio_tag(&tag, active_stream_format)?
                    {
                        current_frame
                            .audio_events
                            .push(FrameAudioEvent::StartSound2 {
                                class_name: ss2.class_name.to_owned(),
                                info: ss2.info,
                            });
                    }
                }
                SwfTagKind::SoundStreamBlock => {
                    let fmt = active_stream_format.ok_or(PlayerHostError::Audio(
                        flash_audio_bridge::AudioBridgeError::StreamFormatMismatch,
                    ))?;
                    let _validated = parse_sound_stream_block(tag.data, fmt)?;
                    current_frame.stream_blocks.push(tag.data.to_vec());
                }
                SwfTagKind::PlaceObject
                | SwfTagKind::PlaceObject2
                | SwfTagKind::PlaceObject3
                | SwfTagKind::RemoveObject
                | SwfTagKind::RemoveObject2 => {
                    let dl_tag = parse_display_list_tag(tag.header.kind.tag_code(), tag.data)?;
                    current_frame.display_ops.push(dl_tag);
                }
                SwfTagKind::DoAction => {
                    if total_scripts >= MAX_FRAME_SCRIPTS {
                        return Err(PlayerHostError::FrameScriptLimitExceeded);
                    }
                    let frame_num = timeline_frames.len() + 1;
                    let mod_name = format!("frame_{frame_num}_doaction_{total_scripts}");
                    let module = lift_avm1(tag.data, &mod_name)?;
                    current_frame.scripts.push(module);
                    total_scripts += 1;
                }
                SwfTagKind::DoInitAction => {
                    if tag.data.len() < 2 {
                        return Err(PlayerHostError::MalformedTag(
                            "DoInitAction shorter than 2 bytes",
                        ));
                    }
                    if total_scripts >= MAX_FRAME_SCRIPTS {
                        return Err(PlayerHostError::FrameScriptLimitExceeded);
                    }
                    let frame_num = timeline_frames.len() + 1;
                    let mod_name = format!("frame_{frame_num}_doinitaction_{total_scripts}");
                    let module = lift_avm1(&tag.data[2..], &mod_name)?;
                    current_frame.scripts.push(module);
                    total_scripts += 1;
                }
                SwfTagKind::DoABC => {
                    if total_scripts >= MAX_FRAME_SCRIPTS {
                        return Err(PlayerHostError::FrameScriptLimitExceeded);
                    }
                    let abc_slice = extract_abc_payload(tag.data)?;
                    let frame_num = timeline_frames.len() + 1;
                    let mod_name = format!("frame_{frame_num}_doabc_{total_scripts}");
                    let module = lift_avm2(abc_slice, &mod_name)?;
                    current_frame.scripts.push(module);
                    total_scripts += 1;
                }
                _ => {}
            }
        }

        if timeline_frames.is_empty()
            && (!current_frame.display_ops.is_empty()
                || !current_frame.scripts.is_empty()
                || current_frame.background_color.is_some()
                || !current_frame.audio_events.is_empty()
                || !current_frame.stream_blocks.is_empty())
        {
            timeline_frames.push(current_frame);
        }

        let total_frames = (timeline_frames.len() as u32)
            .max(header.header_frame_count() as u32)
            .max(1);
        stage.root.total_frames = total_frames;

        let last_scene = PaintScene::new(width_px, height_px);

        Ok(Self {
            config,
            stage,
            audio,
            clock,
            keyboard: KeyboardState::kbd_empty(),
            avm_runner: AvmRunner::runner_new(),
            shapes,
            sprites,
            timeline_frames,
            active_stream_format,
            last_scene,
            pending_initial_frame: true,
            stage_dirty: true,
            close_requested: false,
            processed_action_log_len: 0,
            audit_log: SecurityAuditLog::log_empty(),
            navigation_log: Vec::new(),
            frame_rate_fps: raw_fps,
            declared_frame_count: header.header_frame_count(),
        })
    }

    /// Borrows the underlying `Stage`.
    #[inline]
    pub fn player_stage(&self) -> &Stage {
        &self.stage
    }

    /// Mutably borrows the underlying `Stage`.
    #[inline]
    pub fn player_stage_mut(&mut self) -> &mut Stage {
        self.stage_dirty = true;
        &mut self.stage
    }

    /// Borrows the underlying `FlashAudioState`.
    #[inline]
    pub fn player_audio(&self) -> &FlashAudioState {
        &self.audio
    }

    /// Mutably borrows the underlying `FlashAudioState`.
    #[inline]
    pub fn player_audio_mut(&mut self) -> &mut FlashAudioState {
        &mut self.audio
    }

    /// Borrows the deterministic `GameClockState`.
    #[inline]
    pub fn player_clock(&self) -> &GameClockState {
        &self.clock
    }

    /// Borrows the `KeyboardState`.
    #[inline]
    pub fn player_keyboard(&self) -> &KeyboardState {
        &self.keyboard
    }

    /// Borrows the most recently lowered `PaintScene`.
    #[inline]
    pub fn player_last_scene(&self) -> &PaintScene {
        &self.last_scene
    }

    /// Lowers the active `Stage` into a fresh `PaintScene` (`P2D00`).
    pub fn player_lower_scene(&self) -> Result<PaintScene, PlayerHostError> {
        Ok(self.stage.stage_lower_to_scene()?)
    }

    /// Returns captured `trace(...)` output from executed ActionScript frame scripts.
    pub fn player_trace_output(&self) -> String {
        self.avm_runner.runner_output()
    }

    /// Borrows recorded security diagnostic audit events.
    pub fn player_audit_events(&self) -> &[SecurityAuditEvent] {
        self.audit_log.log_events()
    }

    /// Borrows sanitized navigation requests accepted by `getURL` / `navigateToURL`.
    pub fn player_navigation_requests(&self) -> &[NavigationRequest] {
        &self.navigation_log
    }

    /// Returns `true` if a `WindowEvent::CloseRequested` or `Destroyed` has been received.
    #[inline]
    pub fn player_close_requested(&self) -> bool {
        self.close_requested
    }

    /// Returns the active 1-based root timeline frame index.
    #[inline]
    pub fn player_timeline_frame(&self) -> u32 {
        self.stage.root.clip_current_frame()
    }

    /// Returns total timeline frames on the root clip.
    #[inline]
    pub fn player_total_frames(&self) -> u32 {
        self.stage.root.clip_total_frames()
    }

    /// Returns the configured playback frame rate in FPS.
    #[inline]
    pub fn player_frame_rate_fps(&self) -> f64 {
        self.frame_rate_fps
    }

    /// Returns the number of `DefineShape*` definitions loaded from the SWF.
    #[inline]
    pub fn player_shape_count(&self) -> usize {
        self.shapes.len()
    }

    /// Returns the number of `DefineSprite` definitions loaded from the SWF.
    #[inline]
    pub fn player_sprite_count(&self) -> usize {
        self.sprites.len()
    }

    /// Returns the header-declared `FrameCount`.
    #[inline]
    pub fn player_declared_frame_count(&self) -> u16 {
        self.declared_frame_count
    }

    /// Dispatches a normalized `UI11` (`window_core::WindowEvent`) to `Stage` and [`KeyboardState`].
    pub fn player_dispatch_window_event(
        &mut self,
        event: &WindowEvent,
    ) -> Result<HostInputDispatchResult, PlayerHostError> {
        let res = dispatch_window_event_to_stage(&mut self.stage, &mut self.keyboard, event)?;
        if res.close_requested {
            self.close_requested = true;
        }
        if !res.display_events.is_empty() || res.redraw_requested {
            self.stage_dirty = true;
        }
        Ok(res)
    }

    /// Dispatches `fscommand(command, args)` under strict CWE-78 confinement.
    ///
    /// `fscommand("exec", ...)` is permanently disabled as a no-op and records a
    /// [`SecurityAuditKind::FsCommandExecBlocked`] event. Zero ambient authority.
    pub fn player_dispatch_fscommand(&mut self, command: &str, args: &str) -> FsCommandOutcome {
        let outcome = evaluate_fscommand(command, args);
        let frame = self.stage.root.clip_current_frame();
        match outcome {
            FsCommandOutcome::BlockedNoOp => {
                self.audit_log.log_record(
                    frame,
                    SecurityAuditKind::FsCommandExecBlocked,
                    &format!("blocked fscommand({command})"),
                );
            }
            FsCommandOutcome::HandledNoOp => {
                self.audit_log.log_record(
                    frame,
                    SecurityAuditKind::FsCommandHandled,
                    &format!("handled fscommand({command})"),
                );
            }
        }
        outcome
    }

    /// Validates and records a `getURL` / `navigateToURL` request under CWE-94 / CWE-78 policy.
    ///
    /// - Intercepts `fscommand:` pseudo-URLs and routes them through [`FlashPlayer::player_dispatch_fscommand`],
    ///   permanently blocking `fscommand:exec` with [`PlayerHostError::BlockedFsCommand`].
    /// - Strictly sanitizes URI schemes via [`sanitize_navigation_uri`], permitting only
    ///   `http:`, `https:`, or relative URIs while blocking `javascript:`, `file:`, `data:`, `vbscript:`.
    pub fn player_navigate_url<'a>(
        &mut self,
        raw_uri: &'a str,
        target: &str,
    ) -> Result<&'a str, PlayerHostError> {
        let frame = self.stage.root.clip_current_frame();
        let trimmed = trim_whitespace_and_control(raw_uri);

        if trimmed.len() >= 10 && trimmed[..10].eq_ignore_ascii_case("fscommand:") {
            let cmd = &trimmed[10..];
            if matches!(
                self.player_dispatch_fscommand(cmd, target),
                FsCommandOutcome::BlockedNoOp
            ) {
                return Err(PlayerHostError::BlockedFsCommand);
            }
            return Ok(trimmed);
        }

        match sanitize_navigation_uri(raw_uri) {
            Ok(clean) => {
                self.audit_log
                    .log_record(frame, SecurityAuditKind::UriAllowed, clean);
                if self.navigation_log.len() >= MAX_NAVIGATION_LOG {
                    self.navigation_log.remove(0);
                }
                self.navigation_log.push(NavigationRequest {
                    uri: clean.to_string(),
                    target: target.to_string(),
                    frame,
                });
                Ok(clean)
            }
            Err(err) => {
                self.audit_log
                    .log_record(frame, SecurityAuditKind::UriBlocked, trimmed);
                Err(err)
            }
        }
    }

    /// Compiles and executes an inline ActionScript source snippet via `actionscript-compiler` (`SWF03`)
    /// with the host's bounded `vm-core` step limit (`max_steps <= 2_000_000`).
    pub fn player_execute_source_script(
        &mut self,
        source: &str,
        dialect: AsDialect,
    ) -> Result<(), PlayerHostError> {
        let module = compile_actionscript(source, dialect, "host_inline_script")?;
        self.execute_iir_module_bounded(&module)
    }

    /// Advances the Flash player loop using `game-clock` (`GAME10`) for `raw_delta_ns` nanoseconds:
    /// 1. Advances fixed-timestep accumulator (`advance_frame`).
    /// 2. For each step, applies timeline frame tags (`PlaceObject*`/`RemoveObject*`/`SetBackgroundColor`),
    ///    runs lifted `DoAction`/`DoABC` scripts (`max_steps <= 2_000_000`), syncs `flash-audio-bridge`,
    ///    and ticks `Stage::stage_advance_frame`.
    /// 3. Renders pending PCM samples if `out_audio_pcm` is `Some`.
    /// 4. Flattens `Stage` to `PaintScene` and diffs via `paint-palette-blend::diff_paint_scene_into`.
    pub fn player_step_with_clock(
        &mut self,
        raw_delta_ns: u64,
        out_patches: &mut [PatchOp],
        out_audio_pcm: Option<&mut [i16]>,
    ) -> Result<PlayerTickSummary, PlayerHostError> {
        let step_res = advance_frame(&self.clock, raw_delta_ns)?;
        self.clock = step_res.state;

        for _ in 0..step_res.steps_to_run {
            let active_frame_idx = self.stage.root.clip_current_frame() as usize;
            self.execute_frame_contents(active_frame_idx)?;
            let _events = self.stage.stage_advance_frame()?;
            self.pending_initial_frame = false;
            self.stage_dirty = true;
        }

        let _expected_audio_samples = self.audio.advance_frame_clock(raw_delta_ns)?;
        let audio_samples_rendered = match out_audio_pcm {
            Some(buf) => render_flash_audio_into(&mut self.audio, buf)?,
            None => 0,
        };

        let patch_summary = if self.stage_dirty || self.pending_initial_frame {
            let next_scene = self.stage.stage_lower_to_scene()?;
            let summary = diff_paint_scene_into(&self.last_scene, &next_scene, out_patches)?;
            self.last_scene = next_scene;
            self.stage_dirty = false;
            summary
        } else {
            diff_paint_scene_into(&self.last_scene, &self.last_scene, out_patches)?
        };

        Ok(PlayerTickSummary {
            steps_run: step_res.steps_to_run,
            timeline_frame: self.stage.root.clip_current_frame(),
            patch_summary,
            audio_samples_rendered,
            spiral_of_death_clamped: step_res.spiral_of_death_clamped,
            dropped_ns: step_res.dropped_ns,
        })
    }

    /// Zero-allocation steady-state tick helper (`VER07`) advancing `game-clock`, root timeline,
    /// `flash-audio-bridge` PCM rendering, and `diff_paint_scene_into` without heap allocation.
    pub fn player_step_clock_and_audio_zero_alloc(
        &mut self,
        raw_delta_ns: u64,
        prev_scene: &PaintScene,
        next_scene: &PaintScene,
        out_patches: &mut [PatchOp],
        out_audio_pcm: &mut [i16],
    ) -> Result<PlayerZeroAllocTick, PlayerHostError> {
        let clock_step = advance_frame(&self.clock, raw_delta_ns)?;
        self.clock = clock_step.state;

        for _ in 0..clock_step.steps_to_run {
            self.stage.root.clip_step_timeline();
        }

        let _expected_samples = self.audio.advance_frame_clock(raw_delta_ns)?;
        let audio_samples_rendered = render_flash_audio_into(&mut self.audio, out_audio_pcm)?;
        let patch_summary = diff_paint_scene_into(prev_scene, next_scene, out_patches)?;

        Ok(PlayerZeroAllocTick {
            clock_step,
            timeline_frame: self.stage.root.clip_current_frame(),
            patch_summary,
            audio_samples_rendered,
        })
    }

    /// Renders the current `PaintScene` through a `paint-vm-runtime` backend registry (`P2D01`).
    pub fn player_render_with_registry(
        &self,
        registry: &PaintBackendRegistry<'_>,
        options: PaintRenderOptions,
    ) -> Result<PixelContainer, PlayerHostError> {
        Ok(registry.render_auto(&self.last_scene, options)?)
    }

    /// Computes the canonical lowercase hexadecimal SHA-256 digest of the active `PaintScene`.
    pub fn player_scene_digest_hex(&self) -> String {
        compute_scene_digest_hex(&self.last_scene)
    }

    fn execute_frame_contents(&mut self, one_based_frame: usize) -> Result<(), PlayerHostError> {
        let frame_opt = one_based_frame
            .checked_sub(1)
            .and_then(|idx| self.timeline_frames.get(idx))
            .cloned();

        let Some(frame_data) = frame_opt else {
            return Ok(());
        };

        if let Some(ref bg) = frame_data.background_color {
            self.stage.stage_set_background_color(bg);
        }

        for dl_tag in &frame_data.display_ops {
            apply_display_tag_to_clip(&mut self.stage.root, dl_tag, &self.shapes, &self.sprites)?;
        }

        for ev in &frame_data.audio_events {
            match ev {
                FrameAudioEvent::StartSound(ss) => {
                    self.audio.apply_start_sound(ss)?;
                }
                FrameAudioEvent::StartSound2 { class_name, info } => {
                    let tag = flash_audio_bridge::StartSound2Tag {
                        class_name: class_name.as_str(),
                        info: *info,
                    };
                    self.audio.apply_start_sound2(&tag)?;
                }
            }
        }

        if let Some(fmt) = self.active_stream_format {
            for raw_block in &frame_data.stream_blocks {
                let block = parse_sound_stream_block(raw_block, fmt)?;
                let _ = self.audio.push_stream_block(&block)?;
            }
        }

        for module in &frame_data.scripts {
            self.execute_iir_module_bounded(module)?;
        }

        Ok(())
    }

    fn execute_iir_module_bounded(&mut self, module: &IIRModule) -> Result<(), PlayerHostError> {
        let frame = self.stage.root.clip_current_frame();
        if self.config.max_vm_steps_per_frame == MAX_HOST_VM_STEPS {
            match self.avm_runner.runner_execute(module) {
                Ok(_) => {}
                Err(LifterError::ScriptTimeout) => {
                    self.audit_log.log_record(
                        frame,
                        SecurityAuditKind::ScriptTimeout,
                        "frame script exceeded MAX_HOST_VM_STEPS",
                    );
                    return Err(PlayerHostError::ScriptTimeout);
                }
                Err(LifterError::PrototypePollutionBlocked(msg)) => {
                    self.audit_log.log_record(
                        frame,
                        SecurityAuditKind::PrototypePollutionBlocked,
                        &msg,
                    );
                    return Err(PlayerHostError::Lifter(
                        LifterError::PrototypePollutionBlocked(msg),
                    ));
                }
                Err(other) => return Err(PlayerHostError::Lifter(other)),
            }
            self.sync_runner_timeline_actions();
        } else {
            let mut vm = vm_core::VMCore::new();
            vm.max_instructions = Some(self.config.max_vm_steps_per_frame as u64);
            let mut mod_clone = module.clone();
            let entry = mod_clone
                .entry_point
                .clone()
                .unwrap_or_else(|| "main".to_string());
            if let Err(err) = vm.execute(&mut mod_clone, &entry, &[]) {
                let msg = format!("{err:?}");
                if msg.contains("execution step limit") {
                    self.audit_log.log_record(
                        frame,
                        SecurityAuditKind::ScriptTimeout,
                        "frame script exceeded custom VM step budget",
                    );
                    return Err(PlayerHostError::ScriptTimeout);
                }
                return Err(PlayerHostError::Lifter(LifterError::ExecutionError(msg)));
            }
        }

        Ok(())
    }

    fn sync_runner_timeline_actions(&mut self) {
        let log = self.avm_runner.runner_action_log();
        if self.processed_action_log_len >= log.len() {
            return;
        }
        for action in &log[self.processed_action_log_len..] {
            match action.as_str() {
                "stop" => self.stage.root.clip_stop(),
                "play" => self.stage.root.clip_play(),
                other if other.starts_with("gotoFrame(") && other.ends_with(')') => {
                    let inner = &other[10..other.len() - 1];
                    if let Ok(zero_based) = inner.parse::<u32>() {
                        let one_based = zero_based
                            .saturating_add(1)
                            .min(self.stage.root.total_frames);
                        let _ = self
                            .stage
                            .root
                            .clip_goto_and_stop(FrameTarget::Index(one_based));
                    }
                }
                _ => {}
            }
        }
        self.processed_action_log_len = log.len();
    }
}

/// Applies a display-list tag to a [`MovieClip`], resolving character definitions from the SWF dictionary.
fn apply_display_tag_to_clip(
    clip: &mut MovieClip,
    dl_tag: &DisplayListTag,
    shapes: &BTreeMap<u16, ParsedShape>,
    sprites: &BTreeMap<u16, MovieClip>,
) -> Result<(), PlayerHostError> {
    match dl_tag {
        DisplayListTag::Place(rec) => {
            if !rec.is_move {
                if let Some(char_id) = rec.character_id {
                    let obj = if let Some(shape) = shapes.get(&char_id) {
                        Some(DisplayObject::Shape(shape.clone()))
                    } else {
                        sprites
                            .get(&char_id)
                            .map(|sp| DisplayObject::Clip(sp.clone()))
                    };
                    if let Some(display_obj) = obj {
                        let mut slot = DisplaySlot::new(rec.depth, display_obj);
                        slot.character_id = Some(char_id);
                        if let Some(m) = rec.matrix {
                            slot.transform = m;
                        }
                        if let Some(cx) = rec.color_transform {
                            slot.color_transform = cx;
                        }
                        if let Some(r) = rec.ratio {
                            slot.ratio = Some(r);
                        }
                        if let Some(ref n) = rec.name {
                            slot.name = Some(n.clone());
                        }
                        if let Some(cd) = rec.clip_depth {
                            slot.clip_depth = Some(cd);
                        }
                        if let Some(bm) = rec.blend_mode {
                            slot.blend_mode = Some(bm);
                        }
                        if let Some(vis) = rec.visible {
                            slot.visible = vis;
                        }
                        clip.clip_insert_slot(slot)?;
                        return Ok(());
                    }
                }
            }
            clip.clip_apply_place_record(rec)?;
        }
        DisplayListTag::Remove(rem) => {
            let _ = clip.clip_apply_remove_record(rem)?;
        }
    }
    Ok(())
}

/// Parses a `DefineSprite` (Tag 39) payload into `(sprite_id, MovieClip)`.
fn parse_define_sprite_clip(
    data: &[u8],
    shapes: &BTreeMap<u16, ParsedShape>,
) -> Result<(u16, MovieClip), PlayerHostError> {
    if data.len() < 4 {
        return Err(PlayerHostError::MalformedTag(
            "DefineSprite shorter than 4 bytes",
        ));
    }
    let sprite_id = u16::from_le_bytes([data[0], data[1]]);
    let frame_count = u16::from_le_bytes([data[2], data[3]]);
    let mut clip = MovieClip::new((frame_count as u32).max(1));
    let empty_sprites = BTreeMap::new();

    for nested_res in SwfTagIter::from_payload(&data[4..]) {
        let nested = nested_res?;
        match nested.header.kind {
            SwfTagKind::End | SwfTagKind::ShowFrame => break,
            SwfTagKind::PlaceObject
            | SwfTagKind::PlaceObject2
            | SwfTagKind::PlaceObject3
            | SwfTagKind::RemoveObject
            | SwfTagKind::RemoveObject2 => {
                let dl_tag = parse_display_list_tag(nested.header.kind.tag_code(), nested.data)?;
                apply_display_tag_to_clip(&mut clip, &dl_tag, shapes, &empty_sprites)?;
            }
            _ => {}
        }
    }

    Ok((sprite_id, clip))
}

/// Extracts raw ABC bytecode from either a raw ABC stream or an SWF `DoABC` (Tag 82) header.
fn extract_abc_payload(data: &[u8]) -> Result<&[u8], PlayerHostError> {
    if data.len() < 4 {
        return Err(PlayerHostError::MalformedTag("DoABC payload too short"));
    }
    // Raw ABC always begins with minor_version=16 (0x10 0x00) and major_version=46 (0x2E 0x00).
    if data[0] == 0x10 && data[1] == 0x00 && data[2] == 0x2E && data[3] == 0x00 {
        return Ok(data);
    }
    // Standard SWF9+ DoABC2 header: 4-byte flags + null-terminated UTF-8 name + ABC bytes.
    let rest = &data[4..];
    let nul_pos = rest
        .iter()
        .position(|&b| b == 0)
        .ok_or(PlayerHostError::MalformedTag(
            "DoABC missing null-terminated name",
        ))?;
    let abc = &rest[nul_pos + 1..];
    if abc.len() < 4 {
        return Err(PlayerHostError::MalformedTag(
            "DoABC body shorter than ABC version header",
        ));
    }
    Ok(abc)
}

/// Windowed + `paint-vm-runtime` integrated host session over any `UI11` [`WindowBackend`].
pub struct FlashHostSession<B: WindowBackend> {
    player: FlashPlayer,
    backend: B,
    window: B::Window,
}

impl<B: WindowBackend> FlashHostSession<B> {
    /// Creates a presentation window matching the loaded SWF stage dimensions and binds it to `player`.
    pub fn session_create(
        player: FlashPlayer,
        mut backend: B,
        title: &str,
    ) -> Result<Self, PlayerHostError> {
        let (w, h) = player.player_stage().stage_dimensions();
        let attrs = WindowBuilder::new()
            .title(title)
            .initial_size(LogicalSize::new(w, h))
            .build()?;
        let window = backend.create_window(attrs)?;
        Ok(Self {
            player,
            backend,
            window,
        })
    }

    /// Pumps `UI11` events from the window backend, dispatches them to `Stage`, and steps `FlashPlayer`.
    pub fn session_pump_and_step(
        &mut self,
        raw_delta_ns: u64,
        out_patches: &mut [PatchOp],
        out_audio_pcm: Option<&mut [i16]>,
    ) -> Result<PlayerTickSummary, PlayerHostError> {
        let events = self.backend.pump_events()?;
        for ev in &events {
            let _ = self.player.player_dispatch_window_event(ev)?;
        }
        self.player
            .player_step_with_clock(raw_delta_ns, out_patches, out_audio_pcm)
    }

    /// Renders the active `PaintScene` via `paint-vm-runtime` (`P2D01`).
    pub fn session_render(
        &self,
        registry: &PaintBackendRegistry<'_>,
        options: PaintRenderOptions,
    ) -> Result<PixelContainer, PlayerHostError> {
        self.player.player_render_with_registry(registry, options)
    }

    /// Borrows the underlying [`FlashPlayer`].
    #[inline]
    pub fn session_player(&self) -> &FlashPlayer {
        &self.player
    }

    /// Mutably borrows the underlying [`FlashPlayer`].
    #[inline]
    pub fn session_player_mut(&mut self) -> &mut FlashPlayer {
        &mut self.player
    }

    /// Borrows the `UI11` window handle.
    #[inline]
    pub fn session_window(&self) -> &B::Window {
        &self.window
    }
}
