//! Deterministic headless CI runner and SHA-256 frame/scene digest verifier (`SWF07`).
//!
//! Executes `.swf` payloads offscreen without requiring a display server, producing
//! canonical SHA-256 digests of emitted `PaintScene` trees and optional PCM audio frames
//! for regression gating (`SWF00` §3.7 / `SWF08`).

#![forbid(unsafe_code)]

use flash_display_list::{PatchKind, PatchOp, PatchSummary};

use crate::digest::compute_pcm_digest_hex;
use crate::error::{PlayerHostError, MAX_HEADLESS_FRAMES};
use crate::player::{FlashPlayer, FlashPlayerConfig};
use crate::security::SecurityAuditEvent;

/// Configuration for deterministic headless SWF execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessRunConfig {
    /// Number of frames to step (`1..=MAX_HEADLESS_FRAMES`).
    pub frame_count: u32,
    /// Optional fixed delta per step in nanoseconds (`None` uses the movie's exact `fixed_dt_ns`).
    pub step_dt_ns: Option<u64>,
    /// Whether to render and SHA-256 hash mono PCM audio samples per frame.
    pub render_audio_per_frame: bool,
    /// Optional `(1-based tick index, expected SHA-256 hex)` digest assertions.
    pub expected_frame_digests: Vec<(u32, String)>,
}

impl HeadlessRunConfig {
    /// Creates a headless run configuration for `frame_count` deterministic frames.
    pub fn headless_cfg_for_frames(frame_count: u32) -> Self {
        Self {
            frame_count,
            step_dt_ns: None,
            render_audio_per_frame: false,
            expected_frame_digests: Vec::new(),
        }
    }
}

/// Deterministic digest record produced for a single stepped frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessFrameDigest {
    /// 1-based execution step counter.
    pub step_index: u32,
    /// Active 1-based root `MovieClip` timeline frame after this step.
    pub timeline_frame: u32,
    /// Canonical lowercase 64-char hex SHA-256 digest of the lowered `PaintScene`.
    pub scene_digest_hex: String,
    /// Number of top-level `PaintInstruction` items in the lowered `PaintScene`.
    pub instruction_count: usize,
    /// Retained-mode diff summary relative to the preceding frame.
    pub patch_summary: PatchSummary,
    /// Optional SHA-256 hex digest of rendered mono `i16` audio samples for this frame.
    pub audio_digest_hex: Option<String>,
}

/// Complete report returned by [`run_headless_swf`].
#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessRunReport {
    /// Stage width in pixels.
    pub stage_width: f64,
    /// Stage height in pixels.
    pub stage_height: f64,
    /// Movie playback frame rate in FPS.
    pub frame_rate_fps: f64,
    /// Per-frame deterministic digests.
    pub frames: Vec<HeadlessFrameDigest>,
    /// Captured `trace(...)` output across all frames.
    pub trace_output: String,
    /// Security audit events recorded during execution.
    pub audit_events: Vec<SecurityAuditEvent>,
}

impl HeadlessRunReport {
    /// Looks up the scene digest hex for a 1-based `step_index`.
    pub fn report_frame_digest(&self, step_index: u32) -> Option<&str> {
        self.frames
            .iter()
            .find(|f| f.step_index == step_index)
            .map(|f| f.scene_digest_hex.as_str())
    }
}

/// Executes an `.swf` binary payload deterministically offscreen without a display server.
pub fn run_headless_swf(
    swf_bytes: &[u8],
    player_cfg: &FlashPlayerConfig,
    run_cfg: &HeadlessRunConfig,
) -> Result<HeadlessRunReport, PlayerHostError> {
    if run_cfg.frame_count == 0 || run_cfg.frame_count > MAX_HEADLESS_FRAMES {
        return Err(PlayerHostError::HeadlessFrameLimitExceeded);
    }

    let mut player = FlashPlayer::player_from_swf_bytes(swf_bytes, player_cfg.clone())?;
    let dt_ns = run_cfg
        .step_dt_ns
        .unwrap_or_else(|| player.player_clock().fixed_dt_ns);

    let (stage_width, stage_height) = player.player_stage().stage_dimensions();
    let frame_rate_fps = player.player_frame_rate_fps();

    let mut patch_buf = vec![
        PatchOp {
            kind: PatchKind::Added,
            old_index: 0,
            new_index: 0,
        };
        4096
    ];

    let samples_per_frame = if run_cfg.render_audio_per_frame {
        let fps = frame_rate_fps.max(1.0);
        ((player.player_audio().engine_output_sample_rate_hz() as f64 / fps).round() as usize)
            .clamp(1, 16_384)
    } else {
        0
    };
    let mut audio_buf = vec![0i16; samples_per_frame];

    let mut frames = Vec::with_capacity(run_cfg.frame_count as usize);

    for step in 1..=run_cfg.frame_count {
        let tick = if run_cfg.render_audio_per_frame {
            player.player_step_with_clock(dt_ns, &mut patch_buf, Some(&mut audio_buf))?
        } else {
            player.player_step_with_clock(dt_ns, &mut patch_buf, None)?
        };

        let scene_digest_hex = player.player_scene_digest_hex();
        let instruction_count = player.player_last_scene().instructions.len();
        let audio_digest_hex = if run_cfg.render_audio_per_frame {
            Some(compute_pcm_digest_hex(
                &audio_buf[..tick.audio_samples_rendered],
            ))
        } else {
            None
        };

        frames.push(HeadlessFrameDigest {
            step_index: step,
            timeline_frame: tick.timeline_frame,
            scene_digest_hex,
            instruction_count,
            patch_summary: tick.patch_summary,
            audio_digest_hex,
        });
    }

    let report = HeadlessRunReport {
        stage_width,
        stage_height,
        frame_rate_fps,
        frames,
        trace_output: player.player_trace_output(),
        audit_events: player.player_audit_events().to_vec(),
    };

    if !run_cfg.expected_frame_digests.is_empty() {
        verify_headless_digests(&report, &run_cfg.expected_frame_digests)?;
    }

    Ok(report)
}

/// Verifies that every `(step_index, expected_hex)` pair matches the produced [`HeadlessRunReport`].
pub fn verify_headless_digests(
    report: &HeadlessRunReport,
    expected: &[(u32, String)],
) -> Result<(), PlayerHostError> {
    for (frame, expected_hex) in expected {
        let actual =
            report
                .report_frame_digest(*frame)
                .ok_or_else(|| PlayerHostError::DigestMismatch {
                    frame: *frame,
                    expected: expected_hex.clone(),
                    actual: "<missing frame>".to_string(),
                })?;
        if !actual.eq_ignore_ascii_case(expected_hex) {
            return Err(PlayerHostError::DigestMismatch {
                frame: *frame,
                expected: expected_hex.clone(),
                actual: actual.to_string(),
            });
        }
    }
    Ok(())
}
