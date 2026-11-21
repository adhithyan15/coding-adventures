//! Synthetic clean-room SWF conformance corpus cases, adversarial security corpus,
//! and pinned SHA-256 digest runner (`SWF08`).

#![forbid(unsafe_code)]

use avm_bytecode_lifter::{lift_avm2, AvmRunner, LifterError};
use flash_display_list::{
    DisplayListError, DisplayListEvent, DisplayObject, DisplaySlot, FrameTarget, MovieClip,
    PaintInstruction, PatchKind, PatchOp, PatchSummary, SimpleButton,
};
use flash_player_host::{
    compute_pcm_digest_hex, compute_scene_digest_hex, run_headless_swf, verify_headless_digests,
    ElementState, FlashPlayer, FlashPlayerConfig, FsCommandOutcome, HeadlessFrameDigest,
    HeadlessRunConfig, HeadlessRunReport, PlayerHostError, PointerButton, SecurityAuditKind,
    WindowEvent, WindowId,
};
use swf_container::{SwfError, SwfMatrix, SwfMovie, SwfRect, SwfTagKind};
use swf_shape_to_paint::{parse_define_shape, ParsedShape};
use vm_core::value::Value;

use crate::builder::{
    encode_swf_tag, write_abc_u30, Avm1ActionBuilder, PlaceTagSpec, ShapeDefSpec, ShapeFillSpec,
    ShapeLineSpec, ShapeRecordSpec, SwfBuilder, SwfCompression,
};
use crate::error::CorpusError;
use crate::validate_corpus_digest_hex;

/// Identifiers for the 6 mandatory `SWF08` positive conformance corpus cases (`SWF00` §3.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CorpusId {
    /// `corpus-01-shapes`: straight edges, quadratic Béziers, linear/radial gradients, hole cutouts.
    Shapes,
    /// `corpus-02-timeline`: `CWS` multi-frame timeline, `PlaceObject2`/`RemoveObject2`, nested `DefineSprite`, `gotoAndPlay`/`gotoAndStop`.
    Timeline,
    /// `corpus-03-buttons`: `SimpleButton` stage hit-testing and `Up`/`Over`/`Down` transitions.
    Buttons,
    /// `corpus-04-avm1-math`: `DoAction` AVM1 arithmetic, variable scoping, branches, `DefineFunction`/`DefineFunction2`, `trace`.
    Avm1Math,
    /// `corpus-05-avm2-oop`: `DoABC` AVM2 constant pool, class hierarchy, methods, exception table, `vm-core` execution.
    Avm2Oop,
    /// `corpus-06-streaming-audio`: frame-by-frame sync of `SoundStreamHead2` + ADPCM `SoundStreamBlock` & `DefineSound`/`StartSound`.
    StreamingAudio,
}

impl CorpusId {
    /// Returns all 6 mandatory positive corpus case identifiers in canonical order.
    pub const fn corpus_id_all() -> &'static [CorpusId] {
        &[
            CorpusId::Shapes,
            CorpusId::Timeline,
            CorpusId::Buttons,
            CorpusId::Avm1Math,
            CorpusId::Avm2Oop,
            CorpusId::StreamingAudio,
        ]
    }

    /// Returns the canonical slug (`"corpus-01-shapes"` .. `"corpus-06-streaming-audio"`).
    pub const fn corpus_id_slug(self) -> &'static str {
        match self {
            Self::Shapes => "corpus-01-shapes",
            Self::Timeline => "corpus-02-timeline",
            Self::Buttons => "corpus-03-buttons",
            Self::Avm1Math => "corpus-04-avm1-math",
            Self::Avm2Oop => "corpus-05-avm2-oop",
            Self::StreamingAudio => "corpus-06-streaming-audio",
        }
    }
}

/// Identifiers for the adversarial security corpus cases (`SWF00` §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdversarialCaseId {
    /// CWE-409: `CWS` zlib payload exceeding 50:1 expansion ratio beyond 1 MB.
    DecompressionBomb,
    /// CWE-400: AVM1 infinite branch loop hitting `max_steps` (`ScriptTimeout`).
    InfiniteAvmLoop,
    /// CWE-400: Display hierarchy nested `> 64` levels (`HierarchyDepthExceeded`).
    DeepSpriteNesting,
    /// CWE-125: SWF tag declaring length past EOF (`UnexpectedEof`).
    TruncatedTag,
    /// CWE-125: AVM1/AVM2 branch landing inside multi-byte instruction (`InvalidBranchTarget`).
    InvalidBranchTarget,
    /// CWE-125: AVM1/AVM2 popping an empty operand stack (`StackUnderflow`).
    StackUnderflow,
    /// CWE-1321: Attempting to overwrite `Object.prototype` (`PrototypePollutionBlocked`).
    PrototypePollution,
    /// CWE-78: `fscommand("exec", ...)` permanently disabled (`BlockedNoOp` + audit event).
    FsCommandExec,
    /// CWE-94: `javascript:`, `file:`, `data:`, `vbscript:` URI rejection (`BlockedUriScheme`).
    DangerousUriSchemes,
}

impl AdversarialCaseId {
    /// Returns all 9 adversarial security corpus cases in canonical order.
    pub const fn adversarial_id_all() -> &'static [AdversarialCaseId] {
        &[
            Self::DecompressionBomb,
            Self::InfiniteAvmLoop,
            Self::DeepSpriteNesting,
            Self::TruncatedTag,
            Self::InvalidBranchTarget,
            Self::StackUnderflow,
            Self::PrototypePollution,
            Self::FsCommandExec,
            Self::DangerousUriSchemes,
        ]
    }

    /// Returns the canonical slug for an adversarial corpus case.
    pub const fn adversarial_id_slug(self) -> &'static str {
        match self {
            Self::DecompressionBomb => "adv-01-cwe409-decompression-bomb",
            Self::InfiniteAvmLoop => "adv-02-cwe400-infinite-avm-loop",
            Self::DeepSpriteNesting => "adv-03-cwe400-deep-sprite-nesting",
            Self::TruncatedTag => "adv-04-cwe125-truncated-tag",
            Self::InvalidBranchTarget => "adv-05-cwe125-invalid-branch-target",
            Self::StackUnderflow => "adv-06-cwe125-stack-underflow",
            Self::PrototypePollution => "adv-07-cwe1321-prototype-pollution",
            Self::FsCommandExec => "adv-08-cwe78-fscommand-exec",
            Self::DangerousUriSchemes => "adv-09-cwe94-dangerous-uris",
        }
    }
}

/// Metadata and pinned canonical SHA-256 expectations for a positive corpus case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusEntry {
    pub id: CorpusId,
    pub slug: &'static str,
    pub description: &'static str,
    pub swf_version: u8,
    pub frame_rate_fps: u8,
    pub frame_count: u32,
    pub render_audio: bool,
    pub expected_frame_digests: Vec<(u32, String)>,
    pub expected_audio_digests: Vec<(u32, String)>,
    pub expected_trace_substring: Option<String>,
}

impl CorpusEntry {
    /// Returns the case identifier.
    #[inline]
    pub const fn entry_id(&self) -> CorpusId {
        self.id
    }

    /// Returns the case slug.
    #[inline]
    pub const fn entry_slug(&self) -> &'static str {
        self.slug
    }

    /// Returns the expected number of stepped frames.
    #[inline]
    pub const fn entry_frame_count(&self) -> u32 {
        self.frame_count
    }

    /// Borrows the pinned `(step_index, sha256_hex)` scene digest pairs.
    #[inline]
    pub fn entry_expected_digests(&self) -> &[(u32, String)] {
        &self.expected_frame_digests
    }
}

/// Result produced by running a single positive corpus case.
#[derive(Debug, Clone, PartialEq)]
pub struct CorpusCaseReport {
    pub id: CorpusId,
    pub slug: &'static str,
    pub swf_bytes_len: usize,
    pub headless_report: HeadlessRunReport,
    pub patch_summaries: Vec<PatchSummary>,
    pub button_events: Vec<DisplayListEvent>,
    pub avm2_return_value: Option<i64>,
    pub digests_verified: bool,
}

impl CorpusCaseReport {
    /// Returns the case identifier.
    #[inline]
    pub const fn case_report_id(&self) -> CorpusId {
        self.id
    }

    /// Looks up the lowercase SHA-256 scene digest hex for a 1-based step index.
    #[inline]
    pub fn case_report_frame_digest(&self, step_index: u32) -> Option<&str> {
        self.headless_report.report_frame_digest(step_index)
    }

    /// Looks up the lowercase SHA-256 PCM audio digest hex for a 1-based step index.
    #[inline]
    pub fn case_report_audio_digest(&self, step_index: u32) -> Option<&str> {
        self.headless_report
            .frames
            .iter()
            .find(|f| f.step_index == step_index)
            .and_then(|f| f.audio_digest_hex.as_deref())
    }

    /// Borrows the captured `trace(...)` output.
    #[inline]
    pub fn case_report_trace(&self) -> &str {
        &self.headless_report.trace_output
    }

    /// Returns the number of executed frame records.
    #[inline]
    pub fn case_report_frame_count(&self) -> usize {
        self.headless_report.frames.len()
    }
}

/// Outcome returned by executing an adversarial security corpus case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdversarialOutcome {
    pub id: AdversarialCaseId,
    pub slug: &'static str,
    pub cwe: &'static str,
    pub blocked: bool,
    pub detail: String,
}

// Pinned canonical SHA-256 scene & audio digests.
const PINNED_SHAPES_F1: &str = "e5aacb68eb8bb16809ab6eaf2a33868bd37d4117eb274122041279937d359e4f";

const PINNED_TIMELINE_F1: &str = "ccf27b616e2c0a0b8b77c319922f84a073e8eda3bb5373f53af1a9043aa44923";
const PINNED_TIMELINE_F2: &str = "ec94e8246e28be0d6af816cd888420240a4671303b9d541983f7fb3f99182bc8";
const PINNED_TIMELINE_F3: &str = "a55ad45a283b8eb93272735e14dec0e7fff94fbb04152c6811dbbf068233d721";
const PINNED_TIMELINE_F4: &str = "aae1af2a09b1612c28a524f5c090ba460ba854fee44d1ead059982e34024861e";

const PINNED_BUTTONS_F1: &str = "228794b41581351f18a04f04188e9bdfd504147b152bb554ca6d2a399fa81efc";
const PINNED_BUTTONS_F2: &str = "f1cc82ee120b35ed276e256e8c77c33f773979ec911b69f6ebf512935ec3f8b6";
const PINNED_BUTTONS_F3: &str = "39418b65a833fd6907a45a91b736eae05e16e319bf07baafdf4a2714d23941b7";
const PINNED_BUTTONS_F4: &str = "f1cc82ee120b35ed276e256e8c77c33f773979ec911b69f6ebf512935ec3f8b6";
const PINNED_BUTTONS_F5: &str = "228794b41581351f18a04f04188e9bdfd504147b152bb554ca6d2a399fa81efc";

const PINNED_AVM1_F1: &str = "26b151fb7b28bc325354d5237c02d622a5fa6a8e69c5c8fb201458a99eeab45d";
const PINNED_AVM2_F1: &str = "192adee6b185d7e75f6e362904ccfdd20af2071efa4398e7fced2c43d95f5b74";

const PINNED_AUDIO_SCENE_F1: &str =
    "9fd9fe3c84937c602467d9eb666dbad70d5fbdf30ec9184f3d3d02b1e32efb4e";
const PINNED_AUDIO_SCENE_F2: &str =
    "a64d6a851620a93400458fab1d44d65360dc713d58de1fdf3933560c0fac03a1";
const PINNED_AUDIO_SCENE_F3: &str =
    "f9fcd17b0075c21d41b09642b81aa2bd5aec0f10708b817da56296a817d6b3cc";

const PINNED_AUDIO_PCM_F1: &str =
    "37babcf6af981f5a77c94272542e0b6ffebe7ae94c37f694c0425f8c2fece634";
const PINNED_AUDIO_PCM_F2: &str =
    "a3b0aadfb5ce3bf77be8d869ccded6ad54444c9642939d70704836b036912a27";
const PINNED_AUDIO_PCM_F3: &str =
    "f143ffbb23bfd6e0bc9c310475101e8063fdfbc5beb46a63ac1b5768a6e1c057";

/// Deterministic runner executing and verifying the `SWF08` conformance & adversarial corpus.
#[derive(Debug, Clone)]
pub struct CorpusRunner {
    player_config: FlashPlayerConfig,
}

impl Default for CorpusRunner {
    fn default() -> Self {
        Self::runner_new()
    }
}

impl CorpusRunner {
    /// Creates a `CorpusRunner` with the standard hardened `FlashPlayerConfig`.
    pub fn runner_new() -> Self {
        Self {
            player_config: FlashPlayerConfig::cfg_standard(),
        }
    }

    /// Creates a `CorpusRunner` with a custom validated `FlashPlayerConfig`.
    pub fn runner_with_config(player_config: FlashPlayerConfig) -> Result<Self, CorpusError> {
        player_config.cfg_validate()?;
        Ok(Self { player_config })
    }

    /// Returns the canonical [`CorpusEntry`] definition and pinned digests for `id`.
    pub fn runner_entry(id: CorpusId) -> CorpusEntry {
        match id {
            CorpusId::Shapes => CorpusEntry {
                id,
                slug: id.corpus_id_slug(),
                description: "DefineShape1..4 straight/curved quadratic Bezier edges, linear/radial gradients, and inner-island hole cutouts",
                swf_version: 8,
                frame_rate_fps: 24,
                frame_count: 1,
                render_audio: false,
                expected_frame_digests: vec![(1, PINNED_SHAPES_F1.to_string())],
                expected_audio_digests: Vec::new(),
                expected_trace_substring: None,
            },
            CorpusId::Timeline => CorpusEntry {
                id,
                slug: id.corpus_id_slug(),
                description: "CWS compressed multi-frame timeline, PlaceObject2/RemoveObject2 transforms, DefineSprite nested clip, gotoAndPlay/gotoAndStop",
                swf_version: 8,
                frame_rate_fps: 24,
                frame_count: 4,
                render_audio: false,
                expected_frame_digests: vec![
                    (1, PINNED_TIMELINE_F1.to_string()),
                    (2, PINNED_TIMELINE_F2.to_string()),
                    (3, PINNED_TIMELINE_F3.to_string()),
                    (4, PINNED_TIMELINE_F4.to_string()),
                ],
                expected_audio_digests: Vec::new(),
                expected_trace_substring: None,
            },
            CorpusId::Buttons => CorpusEntry {
                id,
                slug: id.corpus_id_slug(),
                description: "Interactive SimpleButton stage hit-testing and Up/Over/Down/Over/Up cursor transitions",
                swf_version: 8,
                frame_rate_fps: 24,
                frame_count: 5,
                render_audio: false,
                expected_frame_digests: vec![
                    (1, PINNED_BUTTONS_F1.to_string()),
                    (2, PINNED_BUTTONS_F2.to_string()),
                    (3, PINNED_BUTTONS_F3.to_string()),
                    (4, PINNED_BUTTONS_F4.to_string()),
                    (5, PINNED_BUTTONS_F5.to_string()),
                ],
                expected_audio_digests: Vec::new(),
                expected_trace_substring: None,
            },
            CorpusId::Avm1Math => CorpusEntry {
                id,
                slug: id.corpus_id_slug(),
                description: "AVM1 DoAction arithmetic, variable scoping, conditional branching, DefineFunction/DefineFunction2, and deterministic trace",
                swf_version: 8,
                frame_rate_fps: 24,
                frame_count: 1,
                render_audio: false,
                expected_frame_digests: vec![(1, PINNED_AVM1_F1.to_string())],
                expected_audio_digests: Vec::new(),
                expected_trace_substring: Some("sum=42\ndiff=18\nprod=72\nquot=6\nless=true\neq=true\n".to_string()),
            },
            CorpusId::Avm2Oop => CorpusEntry {
                id,
                slug: id.corpus_id_slug(),
                description: "AVM2 DoABC constant pool, class hierarchy, method bodies, exception table, and deterministic vm-core execution",
                swf_version: 9,
                frame_rate_fps: 24,
                frame_count: 1,
                render_audio: false,
                expected_frame_digests: vec![(1, PINNED_AVM2_F1.to_string())],
                expected_audio_digests: Vec::new(),
                expected_trace_substring: None,
            },
            CorpusId::StreamingAudio => CorpusEntry {
                id,
                slug: id.corpus_id_slug(),
                description: "Frame-by-frame sync of SoundStreamHead2 + 4-bit ADPCM SoundStreamBlock and DefineSound/StartSound",
                swf_version: 8,
                frame_rate_fps: 24,
                frame_count: 3,
                render_audio: true,
                expected_frame_digests: vec![
                    (1, PINNED_AUDIO_SCENE_F1.to_string()),
                    (2, PINNED_AUDIO_SCENE_F2.to_string()),
                    (3, PINNED_AUDIO_SCENE_F3.to_string()),
                ],
                expected_audio_digests: vec![
                    (1, PINNED_AUDIO_PCM_F1.to_string()),
                    (2, PINNED_AUDIO_PCM_F2.to_string()),
                    (3, PINNED_AUDIO_PCM_F3.to_string()),
                ],
                expected_trace_substring: None,
            },
        }
    }

    /// Returns all 6 mandatory positive [`CorpusEntry`] definitions.
    pub fn runner_all_entries() -> Vec<CorpusEntry> {
        CorpusId::corpus_id_all()
            .iter()
            .copied()
            .map(Self::runner_entry)
            .collect()
    }

    /// Synthesizes the clean-room `.swf` binary bytes for `id`.
    pub fn runner_build_swf(id: CorpusId) -> Result<Vec<u8>, CorpusError> {
        match id {
            CorpusId::Shapes => build_corpus_01_shapes_swf(),
            CorpusId::Timeline => build_corpus_02_timeline_swf(),
            CorpusId::Buttons => build_corpus_03_buttons_swf(),
            CorpusId::Avm1Math => build_corpus_04_avm1_math_swf(),
            CorpusId::Avm2Oop => build_corpus_05_avm2_oop_swf(),
            CorpusId::StreamingAudio => build_corpus_06_streaming_audio_swf(),
        }
    }

    /// Executes a single positive corpus case, validates case invariants, and verifies pinned SHA-256 digests.
    pub fn runner_execute_case(&self, id: CorpusId) -> Result<CorpusCaseReport, CorpusError> {
        let entry = Self::runner_entry(id);
        let swf = Self::runner_build_swf(id)?;
        let mut report = match id {
            CorpusId::Buttons => self.execute_buttons_case(&entry, &swf)?,
            CorpusId::Avm2Oop => self.execute_avm2_case(&entry, &swf)?,
            _ => self.execute_standard_case(&entry, &swf)?,
        };
        verify_corpus_digests(&entry, &report)?;
        report.digests_verified = true;
        Ok(report)
    }

    /// Executes and verifies all 6 mandatory positive corpus cases.
    pub fn runner_execute_all(&self) -> Result<Vec<CorpusCaseReport>, CorpusError> {
        let mut reports = Vec::with_capacity(6);
        for &id in CorpusId::corpus_id_all() {
            reports.push(self.runner_execute_case(id)?);
        }
        Ok(reports)
    }

    fn execute_standard_case(
        &self,
        entry: &CorpusEntry,
        swf: &[u8],
    ) -> Result<CorpusCaseReport, CorpusError> {
        let mut run_cfg = HeadlessRunConfig::headless_cfg_for_frames(entry.frame_count);
        run_cfg.render_audio_per_frame = entry.render_audio;
        let headless = run_headless_swf(swf, &self.player_config, &run_cfg)?;
        let patch_summaries = headless.frames.iter().map(|f| f.patch_summary).collect();

        // Case-specific structural assertions beyond raw digests
        match entry.id {
            CorpusId::Shapes => {
                let f1 = headless.frames.first().ok_or_else(|| {
                    CorpusError::UnexpectedCaseFailure("missing frame 1 in shapes".to_string())
                })?;
                if f1.instruction_count < 3 {
                    return Err(CorpusError::UnexpectedCaseFailure(
                        "corpus-01-shapes expected >= 3 top-level PaintInstructions".to_string(),
                    ));
                }
                let player = FlashPlayer::player_from_swf_bytes(swf, self.player_config.clone())?;
                let scene = player.player_lower_scene()?;
                let _ = scene;
                // Verify parsed shapes include QuadTo, gradients, and multi-contour hole cutout
                let swf_movie = SwfMovie::from_swf_bytes(swf, &Default::default())?;
                let mut saw_quad = false;
                let mut saw_linear = false;
                let mut saw_radial = false;
                let mut saw_hole = false;
                for tag_res in swf_movie.movie_tags() {
                    let tag = tag_res?;
                    if matches!(
                        tag.header.kind,
                        SwfTagKind::DefineShape
                            | SwfTagKind::DefineShape3
                            | SwfTagKind::DefineShape4
                    ) {
                        let parsed = parse_define_shape(tag.header.kind.tag_code(), tag.data)?;
                        let lowered = swf_shape_to_paint::lower_shape_to_paint_scene(&parsed);
                        for inst in &lowered.instructions {
                            match inst {
                                PaintInstruction::Path(p) => {
                                    if p.commands.iter().any(|c| {
                                        matches!(c, paint_instructions::PathCommand::QuadTo { .. })
                                    }) {
                                        saw_quad = true;
                                    }
                                    let moves = p
                                        .commands
                                        .iter()
                                        .filter(|c| {
                                            matches!(
                                                c,
                                                paint_instructions::PathCommand::MoveTo { .. }
                                            )
                                        })
                                        .count();
                                    if moves >= 2 {
                                        saw_hole = true;
                                    }
                                }
                                PaintInstruction::Gradient(g) => match g.kind {
                                    paint_instructions::GradientKind::Linear { .. } => {
                                        saw_linear = true;
                                    }
                                    paint_instructions::GradientKind::Radial { .. } => {
                                        saw_radial = true;
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                }
                if !(saw_quad && saw_linear && saw_radial && saw_hole) {
                    return Err(CorpusError::UnexpectedCaseFailure(
                        "corpus-01-shapes missing QuadTo, Linear, Radial, or hole cutout"
                            .to_string(),
                    ));
                }
            }
            CorpusId::Timeline => {
                if headless.frames.len() != 4 {
                    return Err(CorpusError::UnexpectedCaseFailure(
                        "corpus-02-timeline expected 4 frames".to_string(),
                    ));
                }
                if headless.frames[0].patch_summary.added == 0
                    || headless.frames[1].patch_summary.modified == 0
                    || headless.frames[2].patch_summary.removed == 0
                {
                    return Err(CorpusError::UnexpectedCaseFailure(
                        "corpus-02-timeline expected Added -> Modified -> Removed patch deltas"
                            .to_string(),
                    ));
                }
                // Exercise MovieClip gotoAndPlay / gotoAndStop directly on FlashPlayer stage
                let mut player =
                    FlashPlayer::player_from_swf_bytes(swf, self.player_config.clone())?;
                player
                    .player_stage_mut()
                    .stage_root_clip_mut()
                    .clip_goto_and_play(FrameTarget::Index(2))?;
                if player.player_timeline_frame() != 2 {
                    return Err(CorpusError::UnexpectedCaseFailure(
                        "gotoAndPlay(2) did not jump to frame 2".to_string(),
                    ));
                }
                player
                    .player_stage_mut()
                    .stage_root_clip_mut()
                    .clip_goto_and_stop(FrameTarget::Index(3))?;
                if player.player_timeline_frame() != 3 {
                    return Err(CorpusError::UnexpectedCaseFailure(
                        "gotoAndStop(3) did not jump to frame 3".to_string(),
                    ));
                }
            }
            CorpusId::Avm1Math => {
                if let Some(ref expected_trace) = entry.expected_trace_substring {
                    if !headless.trace_output.contains(expected_trace) {
                        return Err(CorpusError::UnexpectedCaseFailure(format!(
                            "corpus-04-avm1-math trace mismatch: got {:?}",
                            headless.trace_output
                        )));
                    }
                }
            }
            CorpusId::StreamingAudio => {
                for f in &headless.frames {
                    if f.audio_digest_hex.is_none() {
                        return Err(CorpusError::UnexpectedCaseFailure(
                            "corpus-06-streaming-audio missing audio_digest_hex".to_string(),
                        ));
                    }
                }
            }
            _ => {}
        }

        Ok(CorpusCaseReport {
            id: entry.id,
            slug: entry.slug,
            swf_bytes_len: swf.len(),
            headless_report: headless,
            patch_summaries,
            button_events: Vec::new(),
            avm2_return_value: None,
            digests_verified: false,
        })
    }

    fn execute_buttons_case(
        &self,
        entry: &CorpusEntry,
        swf: &[u8],
    ) -> Result<CorpusCaseReport, CorpusError> {
        let swf_movie = SwfMovie::from_swf_bytes(swf, &Default::default())?;
        let mut parsed_shapes: Vec<ParsedShape> = Vec::new();
        for tag_res in swf_movie.movie_tags() {
            let tag = tag_res?;
            if tag.header.kind == SwfTagKind::DefineShape {
                parsed_shapes.push(parse_define_shape(2, tag.data)?);
            }
        }
        if parsed_shapes.len() < 4 {
            return Err(CorpusError::UnexpectedCaseFailure(
                "corpus-03-buttons expected 4 state shapes".to_string(),
            ));
        }

        let mut btn = SimpleButton::new();
        btn.button_set_up_record(DisplayObject::Shape(parsed_shapes[0].clone()));
        btn.button_set_over_record(DisplayObject::Shape(parsed_shapes[1].clone()));
        btn.button_set_down_record(DisplayObject::Shape(parsed_shapes[2].clone()));
        btn.button_set_hit_record(DisplayObject::Shape(parsed_shapes[3].clone()));

        let mut player = FlashPlayer::player_from_swf_bytes(swf, self.player_config.clone())?;
        let mut slot = DisplaySlot::new(1, DisplayObject::Button(btn));
        slot.character_id = Some(10);
        player
            .player_stage_mut()
            .stage_root_clip_mut()
            .clip_insert_slot(slot)?;

        let dt_ns = player.player_clock().fixed_dt_ns;
        let mut patch_buf = vec![
            PatchOp {
                kind: PatchKind::Added,
                old_index: 0,
                new_index: 0,
            };
            256
        ];

        let mut frames = Vec::with_capacity(5);
        let mut patch_summaries = Vec::with_capacity(5);
        let mut button_events = Vec::new();

        // Step 1: Up (initial)
        // Step 2: PointerMoved inside hit bounds (40..140, 40..100) -> Over
        // Step 3: PointerButton Primary Pressed -> Down
        // Step 4: PointerButton Primary Released -> Over
        // Step 5: PointerMoved outside hit bounds (5, 5) -> Up
        let step_events: [Option<WindowEvent>; 5] = [
            None,
            Some(WindowEvent::PointerMoved {
                window_id: WindowId(1),
                x: 80.0,
                y: 70.0,
            }),
            Some(WindowEvent::PointerButton {
                window_id: WindowId(1),
                button: PointerButton::Primary,
                state: ElementState::Pressed,
            }),
            Some(WindowEvent::PointerButton {
                window_id: WindowId(1),
                button: PointerButton::Primary,
                state: ElementState::Released,
            }),
            Some(WindowEvent::PointerMoved {
                window_id: WindowId(1),
                x: 5.0,
                y: 5.0,
            }),
        ];

        for (idx, ev_opt) in step_events.iter().enumerate() {
            if let Some(ev) = ev_opt {
                let res = player.player_dispatch_window_event(ev)?;
                button_events.extend(res.display_events);
            }
            let tick = player.player_step_with_clock(dt_ns, &mut patch_buf, None)?;
            let digest = player.player_scene_digest_hex();
            let instruction_count = player.player_last_scene().instructions.len();
            patch_summaries.push(tick.patch_summary);
            frames.push(HeadlessFrameDigest {
                step_index: (idx + 1) as u32,
                timeline_frame: tick.timeline_frame,
                scene_digest_hex: digest,
                instruction_count,
                patch_summary: tick.patch_summary,
                audio_digest_hex: None,
            });
        }

        let (stage_width, stage_height) = player.player_stage().stage_dimensions();
        let headless = HeadlessRunReport {
            stage_width,
            stage_height,
            frame_rate_fps: player.player_frame_rate_fps(),
            frames,
            trace_output: player.player_trace_output(),
            audit_events: player.player_audit_events().to_vec(),
        };

        let btn_action_count = button_events
            .iter()
            .filter(|e| matches!(e, DisplayListEvent::ButtonAction { .. }))
            .count();
        if btn_action_count < 4 {
            return Err(CorpusError::UnexpectedCaseFailure(
                "corpus-03-buttons expected RollOver, Press, Release, RollOut transitions"
                    .to_string(),
            ));
        }

        Ok(CorpusCaseReport {
            id: entry.id,
            slug: entry.slug,
            swf_bytes_len: swf.len(),
            headless_report: headless,
            patch_summaries,
            button_events,
            avm2_return_value: None,
            digests_verified: false,
        })
    }

    fn execute_avm2_case(
        &self,
        entry: &CorpusEntry,
        swf: &[u8],
    ) -> Result<CorpusCaseReport, CorpusError> {
        let run_cfg = HeadlessRunConfig::headless_cfg_for_frames(entry.frame_count);
        let headless = run_headless_swf(swf, &self.player_config, &run_cfg)?;
        let patch_summaries = headless.frames.iter().map(|f| f.patch_summary).collect();

        // Also execute the embedded DoABC payload directly via AvmRunner to inspect return value & method call log
        let swf_movie = SwfMovie::from_swf_bytes(swf, &Default::default())?;
        let mut ret_val = None;
        for tag_res in swf_movie.movie_tags() {
            let tag = tag_res?;
            if tag.header.kind == SwfTagKind::DoABC {
                // Skip 4-byte flags + NUL-terminated name
                let nul_pos = tag.data[4..].iter().position(|&b| b == 0).ok_or_else(|| {
                    CorpusError::UnexpectedCaseFailure("DoABC missing name NUL".to_string())
                })?;
                let abc_slice = &tag.data[4 + nul_pos + 1..];
                let module = lift_avm2(abc_slice, "corpus_05_avm2_oop")?;
                let mut runner = AvmRunner::runner_new();
                let val = runner.runner_execute(&module)?;
                if let Value::Int(n) = val {
                    ret_val = Some(n);
                }
                if !runner
                    .runner_action_log()
                    .iter()
                    .any(|a| a == "callPropVoid(increment)")
                {
                    return Err(CorpusError::UnexpectedCaseFailure(
                        "corpus-05-avm2-oop expected callPropVoid(increment) action log"
                            .to_string(),
                    ));
                }
            }
        }
        if ret_val != Some(42) {
            return Err(CorpusError::UnexpectedCaseFailure(format!(
                "corpus-05-avm2-oop expected return value 42, got {ret_val:?}"
            )));
        }

        Ok(CorpusCaseReport {
            id: entry.id,
            slug: entry.slug,
            swf_bytes_len: swf.len(),
            headless_report: headless,
            patch_summaries,
            button_events: Vec::new(),
            avm2_return_value: ret_val,
            digests_verified: false,
        })
    }

    /// Executes a single adversarial security corpus case and confirms the defense triggers cleanly.
    pub fn runner_execute_adversarial(
        &self,
        adv: AdversarialCaseId,
    ) -> Result<AdversarialOutcome, CorpusError> {
        let slug = adv.adversarial_id_slug();
        match adv {
            AdversarialCaseId::DecompressionBomb => {
                // Fixed-Huffman CWS stream inflating > 1 MB of zeros at > 150:1 ratio (instant in debug & release)
                let bomb_swf = build_cws_decompression_bomb_swf(1_200_000)?;
                let res = FlashPlayer::player_from_swf_bytes(&bomb_swf, self.player_config.clone());
                match res {
                    Err(PlayerHostError::SwfContainer(SwfError::DecompressionBombDetected)) => {
                        Ok(AdversarialOutcome {
                            id: adv,
                            slug,
                            cwe: "CWE-409",
                            blocked: true,
                            detail: "DecompressionBombDetected".to_string(),
                        })
                    }
                    other => Err(CorpusError::UnexpectedCaseFailure(format!(
                        "expected DecompressionBombDetected, got {:?}",
                        other.err()
                    ))),
                }
            }
            AdversarialCaseId::InfiniteAvmLoop => {
                let mut avm = Avm1ActionBuilder::avm1_new();
                avm.avm1_branch_always(-5);
                let actions = avm.avm1_finish();
                let mut builder =
                    SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 200, 200, 24)?;
                builder.swf_builder_do_action(&actions)?;
                builder.swf_builder_show_frame()?;
                let swf = builder.swf_builder_finish()?;

                let mut cfg = self.player_config.clone();
                cfg.max_vm_steps_per_frame = 250;
                let mut player = FlashPlayer::player_from_swf_bytes(&swf, cfg)?;
                let dt_ns = player.player_clock().fixed_dt_ns;
                let mut ops = [PatchOp {
                    kind: PatchKind::Added,
                    old_index: 0,
                    new_index: 0,
                }; 8];
                let err = player
                    .player_step_with_clock(dt_ns, &mut ops, None)
                    .unwrap_err();
                let has_audit = player
                    .player_audit_events()
                    .iter()
                    .any(|e| e.kind == SecurityAuditKind::ScriptTimeout);
                if err == PlayerHostError::ScriptTimeout && has_audit {
                    Ok(AdversarialOutcome {
                        id: adv,
                        slug,
                        cwe: "CWE-400",
                        blocked: true,
                        detail: "ScriptTimeout + audit event".to_string(),
                    })
                } else {
                    Err(CorpusError::UnexpectedCaseFailure(format!(
                        "expected ScriptTimeout, got {err}"
                    )))
                }
            }
            AdversarialCaseId::DeepSpriteNesting => {
                let mut leaf = MovieClip::new(1);
                for _ in 0..66 {
                    let mut parent = MovieClip::new(1);
                    parent.clip_insert_slot(DisplaySlot::new(1, DisplayObject::Clip(leaf)))?;
                    leaf = parent;
                }
                let mut builder =
                    SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 200, 200, 24)?;
                builder.swf_builder_show_frame()?;
                let swf = builder.swf_builder_finish()?;
                let mut player =
                    FlashPlayer::player_from_swf_bytes(&swf, self.player_config.clone())?;
                player
                    .player_stage_mut()
                    .stage_root_clip_mut()
                    .clip_insert_slot(DisplaySlot::new(1, DisplayObject::Clip(leaf)))?;
                let dt_ns = player.player_clock().fixed_dt_ns;
                let mut ops = [PatchOp {
                    kind: PatchKind::Added,
                    old_index: 0,
                    new_index: 0,
                }; 8];
                let err = player
                    .player_step_with_clock(dt_ns, &mut ops, None)
                    .unwrap_err();
                if matches!(
                    err,
                    PlayerHostError::DisplayList(DisplayListError::HierarchyDepthExceeded)
                ) {
                    Ok(AdversarialOutcome {
                        id: adv,
                        slug,
                        cwe: "CWE-400",
                        blocked: true,
                        detail: "HierarchyDepthExceeded (> 64)".to_string(),
                    })
                } else {
                    Err(CorpusError::UnexpectedCaseFailure(format!(
                        "expected HierarchyDepthExceeded, got {err}"
                    )))
                }
            }
            AdversarialCaseId::TruncatedTag => {
                // Tag claiming length 200 bytes inside an SWF whose body ends after 4 bytes
                let mut builder =
                    SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 200, 200, 24)?;
                builder.swf_builder_show_frame()?;
                let mut swf = builder.swf_builder_finish()?;
                // Truncate the last 6 bytes or inject a truncated long tag header before End
                let insert_at = swf.len() - 2;
                let mut corrupt = swf[..insert_at].to_vec();
                // Short tag code 12 (DoAction), length 50, but only 3 bytes follow
                let word = (12u16 << 6) | 50u16;
                corrupt.extend_from_slice(&word.to_le_bytes());
                corrupt.extend_from_slice(&[0x01, 0x02, 0x03]);
                let new_len = (corrupt.len() as u32).to_le_bytes();
                corrupt[4..8].copy_from_slice(&new_len);
                let _ = &mut swf;
                let err = FlashPlayer::player_from_swf_bytes(&corrupt, self.player_config.clone())
                    .err()
                    .ok_or_else(|| {
                        CorpusError::UnexpectedCaseFailure("truncated tag accepted".to_string())
                    })?;
                if matches!(
                    err,
                    PlayerHostError::SwfContainer(
                        SwfError::UnexpectedEof | SwfError::InvalidTagLength
                    )
                ) {
                    Ok(AdversarialOutcome {
                        id: adv,
                        slug,
                        cwe: "CWE-125",
                        blocked: true,
                        detail: format!("{err}"),
                    })
                } else {
                    Err(CorpusError::UnexpectedCaseFailure(format!(
                        "expected UnexpectedEof/InvalidTagLength, got {err}"
                    )))
                }
            }
            AdversarialCaseId::InvalidBranchTarget => {
                // ActionBranchAlways(+3) jumping into the middle of ActionPush
                let mut avm = Avm1ActionBuilder::avm1_new();
                avm.avm1_branch_always(3);
                avm.avm1_push_i32(42);
                avm.avm1_op(0x26);
                let actions = avm.avm1_finish();
                let mut builder =
                    SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 200, 200, 24)?;
                builder.swf_builder_do_action(&actions)?;
                builder.swf_builder_show_frame()?;
                let swf = builder.swf_builder_finish()?;
                let err = FlashPlayer::player_from_swf_bytes(&swf, self.player_config.clone())
                    .err()
                    .ok_or_else(|| {
                        CorpusError::UnexpectedCaseFailure("invalid branch accepted".to_string())
                    })?;
                if matches!(
                    err,
                    PlayerHostError::Lifter(LifterError::InvalidBranchTarget)
                ) {
                    Ok(AdversarialOutcome {
                        id: adv,
                        slug,
                        cwe: "CWE-125",
                        blocked: true,
                        detail: "InvalidBranchTarget".to_string(),
                    })
                } else {
                    Err(CorpusError::UnexpectedCaseFailure(format!(
                        "expected InvalidBranchTarget, got {err}"
                    )))
                }
            }
            AdversarialCaseId::StackUnderflow => {
                let mut avm = Avm1ActionBuilder::avm1_new();
                avm.avm1_op(0x17); // ActionPop on empty stack
                let actions = avm.avm1_finish();
                let mut builder =
                    SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 200, 200, 24)?;
                builder.swf_builder_do_action(&actions)?;
                builder.swf_builder_show_frame()?;
                let swf = builder.swf_builder_finish()?;
                let err = FlashPlayer::player_from_swf_bytes(&swf, self.player_config.clone())
                    .err()
                    .ok_or_else(|| {
                        CorpusError::UnexpectedCaseFailure("stack underflow accepted".to_string())
                    })?;
                if matches!(err, PlayerHostError::Lifter(LifterError::StackUnderflow)) {
                    Ok(AdversarialOutcome {
                        id: adv,
                        slug,
                        cwe: "CWE-125",
                        blocked: true,
                        detail: "StackUnderflow".to_string(),
                    })
                } else {
                    Err(CorpusError::UnexpectedCaseFailure(format!(
                        "expected StackUnderflow, got {err}"
                    )))
                }
            }
            AdversarialCaseId::PrototypePollution => {
                let mut avm = Avm1ActionBuilder::avm1_new();
                avm.avm1_push_str("Object.prototype")?;
                avm.avm1_push_str("polluted")?;
                avm.avm1_op(0x1D); // ActionSetVariable
                let actions = avm.avm1_finish();
                let mut builder =
                    SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 200, 200, 24)?;
                builder.swf_builder_do_action(&actions)?;
                builder.swf_builder_show_frame()?;
                let swf = builder.swf_builder_finish()?;

                let mut player =
                    FlashPlayer::player_from_swf_bytes(&swf, self.player_config.clone())?;
                let dt_ns = player.player_clock().fixed_dt_ns;
                let mut ops = [PatchOp {
                    kind: PatchKind::Added,
                    old_index: 0,
                    new_index: 0,
                }; 8];
                let err = player
                    .player_step_with_clock(dt_ns, &mut ops, None)
                    .unwrap_err();
                let has_audit = player
                    .player_audit_events()
                    .iter()
                    .any(|e| e.kind == SecurityAuditKind::PrototypePollutionBlocked);
                if matches!(
                    err,
                    PlayerHostError::Lifter(LifterError::PrototypePollutionBlocked(_))
                ) && has_audit
                {
                    Ok(AdversarialOutcome {
                        id: adv,
                        slug,
                        cwe: "CWE-1321",
                        blocked: true,
                        detail: "PrototypePollutionBlocked + audit event".to_string(),
                    })
                } else {
                    Err(CorpusError::UnexpectedCaseFailure(format!(
                        "expected PrototypePollutionBlocked, got {err}"
                    )))
                }
            }
            AdversarialCaseId::FsCommandExec => {
                let swf = Self::runner_build_swf(CorpusId::Shapes)?;
                let mut player =
                    FlashPlayer::player_from_swf_bytes(&swf, self.player_config.clone())?;
                let direct = player.player_dispatch_fscommand("exec", "cmd.exe /c whoami");
                let nav = player.player_navigate_url("fscommand:exec", "/bin/sh");
                let exec_events = player
                    .player_audit_events()
                    .iter()
                    .filter(|e| e.kind == SecurityAuditKind::FsCommandExecBlocked)
                    .count();
                if direct == FsCommandOutcome::BlockedNoOp
                    && nav == Err(PlayerHostError::BlockedFsCommand)
                    && exec_events == 2
                {
                    Ok(AdversarialOutcome {
                        id: adv,
                        slug,
                        cwe: "CWE-78",
                        blocked: true,
                        detail: "fscommand(\"exec\") blocked + 2 audit events".to_string(),
                    })
                } else {
                    Err(CorpusError::UnexpectedCaseFailure(
                        "fscommand(\"exec\") was not blocked as expected".to_string(),
                    ))
                }
            }
            AdversarialCaseId::DangerousUriSchemes => {
                let swf = Self::runner_build_swf(CorpusId::Shapes)?;
                let mut player =
                    FlashPlayer::player_from_swf_bytes(&swf, self.player_config.clone())?;
                for dangerous in [
                    "javascript:alert(1)",
                    "file:///etc/passwd",
                    "data:text/html,<script>alert(1)</script>",
                    "vbscript:msgbox(1)",
                    "//evil.example/spoof",
                    "/\\evil.example/spoof",
                ] {
                    if player.player_navigate_url(dangerous, "_self")
                        != Err(PlayerHostError::BlockedUriScheme)
                    {
                        return Err(CorpusError::UnexpectedCaseFailure(format!(
                            "dangerous URI not blocked: {dangerous}"
                        )));
                    }
                }
                let blocked_count = player
                    .player_audit_events()
                    .iter()
                    .filter(|e| e.kind == SecurityAuditKind::UriBlocked)
                    .count();
                if blocked_count == 6 {
                    Ok(AdversarialOutcome {
                        id: adv,
                        slug,
                        cwe: "CWE-94",
                        blocked: true,
                        detail: "6 dangerous URI schemes blocked + audited".to_string(),
                    })
                } else {
                    Err(CorpusError::UnexpectedCaseFailure(
                        "expected 6 UriBlocked audit events".to_string(),
                    ))
                }
            }
        }
    }

    /// Executes all 9 adversarial security corpus cases.
    pub fn runner_execute_all_adversarial(&self) -> Result<Vec<AdversarialOutcome>, CorpusError> {
        let mut out = Vec::with_capacity(9);
        for &adv in AdversarialCaseId::adversarial_id_all() {
            out.push(self.runner_execute_adversarial(adv)?);
        }
        Ok(out)
    }
}

/// Top-level helper: runs a single mandatory positive corpus case and verifies its pinned digests.
pub fn run_corpus_case(id: CorpusId) -> Result<CorpusCaseReport, CorpusError> {
    CorpusRunner::runner_new().runner_execute_case(id)
}

/// Top-level helper: runs all 6 mandatory positive corpus cases and verifies their pinned digests.
pub fn run_all_corpus_cases() -> Result<Vec<CorpusCaseReport>, CorpusError> {
    CorpusRunner::runner_new().runner_execute_all()
}

/// Verifies that every pinned scene and audio SHA-256 digest in `entry` matches `report`.
pub fn verify_corpus_digests(
    entry: &CorpusEntry,
    report: &CorpusCaseReport,
) -> Result<(), CorpusError> {
    for (_, hex) in &entry.expected_frame_digests {
        validate_corpus_digest_hex(hex.as_bytes())?;
    }
    for (_, hex) in &entry.expected_audio_digests {
        validate_corpus_digest_hex(hex.as_bytes())?;
    }

    verify_headless_digests(&report.headless_report, &entry.expected_frame_digests).map_err(
        |e| match e {
            PlayerHostError::DigestMismatch {
                frame,
                expected,
                actual,
            } => CorpusError::DigestMismatch {
                case_slug: entry.slug,
                frame,
                expected,
                actual,
            },
            other => CorpusError::PlayerHost(other),
        },
    )?;

    for (frame, expected_hex) in &entry.expected_audio_digests {
        let actual =
            report
                .case_report_audio_digest(*frame)
                .ok_or_else(|| CorpusError::DigestMismatch {
                    case_slug: entry.slug,
                    frame: *frame,
                    expected: expected_hex.clone(),
                    actual: "<missing audio frame>".to_string(),
                })?;
        if !actual.eq_ignore_ascii_case(expected_hex) {
            return Err(CorpusError::DigestMismatch {
                case_slug: entry.slug,
                frame: *frame,
                expected: expected_hex.clone(),
                actual: actual.to_string(),
            });
        }
    }

    Ok(())
}

/// Recomputes canonical scene digest hex directly from a `PaintScene` (convenience wrapper).
pub fn corpus_scene_digest_hex(scene: &flash_display_list::PaintScene) -> String {
    compute_scene_digest_hex(scene)
}

/// Recomputes canonical PCM digest hex directly from `i16` samples (convenience wrapper).
pub fn corpus_pcm_digest_hex(samples: &[i16]) -> String {
    compute_pcm_digest_hex(samples)
}

// ---------------------------------------------------------------------------
// Clean-room synthetic SWF builders for the 6 mandatory corpus cases
// ---------------------------------------------------------------------------

fn rect_shape_spec(shape_id: u16, bounds: (i32, i32, i32, i32), rgb: (u8, u8, u8)) -> ShapeDefSpec {
    let (x0, y0, x1, y1) = bounds;
    let (r, g, b) = rgb;
    let w = x1 - x0;
    let h = y1 - y0;
    ShapeDefSpec {
        tag_code: 2,
        shape_id,
        bounds: SwfRect::create_rect(x0, x1, y0, y1),
        uses_non_zero_winding: false,
        fill_styles: vec![ShapeFillSpec::Solid(r, g, b, 255)],
        line_styles: Vec::new(),
        records: vec![
            ShapeRecordSpec::StyleChange {
                move_to: Some((x0, y0)),
                fill_style0: None,
                fill_style1: Some(1),
                line_style: None,
            },
            ShapeRecordSpec::StraightEdge { dx: w, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: h },
            ShapeRecordSpec::StraightEdge { dx: -w, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: -h },
        ],
    }
}

fn build_corpus_01_shapes_swf() -> Result<Vec<u8>, CorpusError> {
    let mut b = SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 400, 300, 24)?;
    b.swf_builder_set_background(0xF5, 0xF5, 0xFA)?;

    // Shape 1 (DefineShape): Straight + Quadratic Bezier arch with stroke
    let shape1 = ShapeDefSpec {
        tag_code: 2,
        shape_id: 1,
        bounds: SwfRect::create_rect(400, 1400, 400, 1200),
        uses_non_zero_winding: false,
        fill_styles: vec![ShapeFillSpec::Solid(0xE0, 0x3C, 0x31, 255)],
        line_styles: vec![ShapeLineSpec {
            width_twips: 40,
            color: (0x22, 0x22, 0x22, 255),
        }],
        records: vec![
            ShapeRecordSpec::StyleChange {
                move_to: Some((400, 400)),
                fill_style0: None,
                fill_style1: Some(1),
                line_style: Some(1),
            },
            ShapeRecordSpec::StraightEdge { dx: 800, dy: 0 },
            ShapeRecordSpec::CurvedEdge {
                control_dx: 200,
                control_dy: 200,
                anchor_dx: -200,
                anchor_dy: 200,
            },
            ShapeRecordSpec::StraightEdge { dx: -800, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: -400 },
        ],
    };
    b.swf_builder_define_shape(&shape1)?;

    // Shape 2 (DefineShape3): Linear gradient + Radial gradient
    let grad_mat_lin = SwfMatrix {
        scale_x: 0.05,
        scale_y: 0.05,
        rotate_skew_0: 0.0,
        rotate_skew_1: 0.0,
        translate_x: 1000,
        translate_y: 2200,
    };
    let grad_mat_rad = SwfMatrix {
        scale_x: 0.04,
        scale_y: 0.04,
        rotate_skew_0: 0.0,
        rotate_skew_1: 0.0,
        translate_x: 2400,
        translate_y: 2200,
    };
    let shape2 = ShapeDefSpec {
        tag_code: 32,
        shape_id: 2,
        bounds: SwfRect::create_rect(400, 3000, 1600, 2800),
        uses_non_zero_winding: false,
        fill_styles: vec![
            ShapeFillSpec::LinearGradient {
                matrix: grad_mat_lin,
                stops: vec![(0, (0x1E, 0x88, 0xE5, 255)), (255, (0x00, 0xE6, 0x76, 255))],
            },
            ShapeFillSpec::RadialGradient {
                matrix: grad_mat_rad,
                stops: vec![(0, (0xFF, 0xEA, 0x00, 255)), (255, (0xD5, 0x00, 0xF9, 255))],
            },
        ],
        line_styles: Vec::new(),
        records: vec![
            ShapeRecordSpec::StyleChange {
                move_to: Some((400, 1600)),
                fill_style0: None,
                fill_style1: Some(1),
                line_style: None,
            },
            ShapeRecordSpec::StraightEdge { dx: 1200, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: 1200 },
            ShapeRecordSpec::StraightEdge { dx: -1200, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: -1200 },
            ShapeRecordSpec::StyleChange {
                move_to: Some((1800, 1600)),
                fill_style0: None,
                fill_style1: Some(2),
                line_style: None,
            },
            ShapeRecordSpec::StraightEdge { dx: 1200, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: 1200 },
            ShapeRecordSpec::StraightEdge { dx: -1200, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: -1200 },
        ],
    };
    b.swf_builder_define_shape(&shape2)?;

    // Shape 3 (DefineShape4): Outer square + inner hole cutout (EvenOdd) + LINESTYLE2
    let shape3 = ShapeDefSpec {
        tag_code: 83,
        shape_id: 3,
        bounds: SwfRect::create_rect(2000, 3600, 400, 1600),
        uses_non_zero_winding: false,
        fill_styles: vec![ShapeFillSpec::Solid(0x2E, 0x7D, 0x32, 255)],
        line_styles: vec![ShapeLineSpec {
            width_twips: 20,
            color: (0x1B, 0x5E, 0x20, 255),
        }],
        records: vec![
            ShapeRecordSpec::StyleChange {
                move_to: Some((2000, 400)),
                fill_style0: None,
                fill_style1: Some(1),
                line_style: Some(1),
            },
            ShapeRecordSpec::StraightEdge { dx: 1200, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: 1200 },
            ShapeRecordSpec::StraightEdge { dx: -1200, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: -1200 },
            // Inner island hole cutout
            ShapeRecordSpec::StyleChange {
                move_to: Some((2300, 700)),
                fill_style0: None,
                fill_style1: Some(1),
                line_style: Some(1),
            },
            ShapeRecordSpec::StraightEdge { dx: 600, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: 600 },
            ShapeRecordSpec::StraightEdge { dx: -600, dy: 0 },
            ShapeRecordSpec::StraightEdge { dx: 0, dy: -600 },
        ],
    };
    b.swf_builder_define_shape(&shape3)?;

    b.swf_builder_place_object(&PlaceTagSpec::place_new(1, 1))?;
    b.swf_builder_place_object(&PlaceTagSpec::place_new(2, 2))?;
    b.swf_builder_place_object(&PlaceTagSpec::place_new(3, 3))?;
    b.swf_builder_show_frame()?;
    b.swf_builder_finish()
}

fn build_corpus_02_timeline_swf() -> Result<Vec<u8>, CorpusError> {
    let mut b = SwfBuilder::swf_builder_new(8, SwfCompression::Cws, 400, 300, 24)?;
    b.swf_builder_set_background(0x10, 0x20, 0x30)?;
    b.swf_builder_define_shape(&rect_shape_spec(1, (0, 0, 400, 400), (0xFF, 0x6D, 0x00)))?;

    // Nested DefineSprite (id=2) placing shape 1 inside sprite timeline
    let mut sprite_tags = Vec::new();
    sprite_tags.extend_from_slice(&crate::builder::encode_place_object_tag(
        &PlaceTagSpec::place_new(1, 1),
    )?);
    sprite_tags.extend_from_slice(&encode_swf_tag(1, &[])?);
    sprite_tags.extend_from_slice(&encode_swf_tag(0, &[])?);
    b.swf_builder_define_sprite(2, 2, &sprite_tags)?;

    // Frame 1: Place shape 1 and sprite 2
    let mut p1 = PlaceTagSpec::place_new(1, 1);
    p1.matrix = Some(SwfMatrix {
        translate_x: 400,
        translate_y: 400,
        ..SwfMatrix::default_matrix()
    });
    let mut p2 = PlaceTagSpec::place_new(2, 2);
    p2.matrix = Some(SwfMatrix {
        translate_x: 2000,
        translate_y: 2000,
        ..SwfMatrix::default_matrix()
    });
    b.swf_builder_place_object(&p1)?;
    b.swf_builder_place_object(&p2)?;
    b.swf_builder_show_frame()?;

    // Frame 2: Move depth 1
    let mut m1 = PlaceTagSpec::place_move(1);
    m1.matrix = Some(SwfMatrix {
        translate_x: 1600,
        translate_y: 800,
        ..SwfMatrix::default_matrix()
    });
    b.swf_builder_place_object(&m1)?;
    b.swf_builder_show_frame()?;

    // Frame 3: Remove depth 1
    b.swf_builder_remove_object(1)?;
    b.swf_builder_show_frame()?;

    // Frame 4: Re-place depth 1 and jump back to frame 1 via ActionGotoFrame(0)
    let mut p4 = PlaceTagSpec::place_new(1, 1);
    p4.matrix = Some(SwfMatrix {
        translate_x: 800,
        translate_y: 800,
        ..SwfMatrix::default_matrix()
    });
    b.swf_builder_place_object(&p4)?;
    let mut avm = Avm1ActionBuilder::avm1_new();
    avm.avm1_goto_frame(0);
    b.swf_builder_do_action(&avm.avm1_finish())?;
    b.swf_builder_show_frame()?;

    b.swf_builder_finish()
}

fn build_corpus_03_buttons_swf() -> Result<Vec<u8>, CorpusError> {
    let mut b = SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 300, 200, 24)?;
    b.swf_builder_set_background(0xEE, 0xEE, 0xEE)?;
    // 4 button state shapes over (40..140 px, 40..100 px) = (800..2800, 800..2000) twips
    b.swf_builder_define_shape(&rect_shape_spec(
        1,
        (800, 800, 2800, 2000),
        (0x19, 0x76, 0xD2),
    ))?; // Up: Blue
    b.swf_builder_define_shape(&rect_shape_spec(
        2,
        (800, 800, 2800, 2000),
        (0x38, 0x8E, 0x3C),
    ))?; // Over: Green
    b.swf_builder_define_shape(&rect_shape_spec(
        3,
        (800, 800, 2800, 2000),
        (0xD3, 0x2F, 0x2F),
    ))?; // Down: Red
    b.swf_builder_define_shape(&rect_shape_spec(
        4,
        (800, 800, 2800, 2000),
        (0x00, 0x00, 0x00),
    ))?; // HitTest
    b.swf_builder_show_frame()?;
    b.swf_builder_finish()
}

fn build_corpus_04_avm1_math_swf() -> Result<Vec<u8>, CorpusError> {
    let mut b = SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 320, 240, 24)?;
    b.swf_builder_define_shape(&rect_shape_spec(
        1,
        (200, 200, 1200, 1200),
        (0x45, 0x27, 0xA0),
    ))?;
    b.swf_builder_place_object(&PlaceTagSpec::place_new(1, 1))?;

    let mut avm = Avm1ActionBuilder::avm1_new();
    // x = 30; y = 12;
    avm.avm1_push_str("x")?;
    avm.avm1_push_i32(30);
    avm.avm1_op(0x1D); // SetVariable
    avm.avm1_push_str("y")?;
    avm.avm1_push_i32(12);
    avm.avm1_op(0x1D);

    // trace("sum=" + (x + y)) -> "sum=42"
    avm.avm1_push_str("sum=")?;
    avm.avm1_push_str("x")?;
    avm.avm1_op(0x1C); // GetVariable
    avm.avm1_push_str("y")?;
    avm.avm1_op(0x1C);
    avm.avm1_op(0x47); // Add2
    avm.avm1_op(0x47); // Add2 string concat
    avm.avm1_op(0x26); // Trace

    // trace("diff=" + (x - y)) -> "diff=18"
    avm.avm1_push_str("diff=")?;
    avm.avm1_push_str("x")?;
    avm.avm1_op(0x1C);
    avm.avm1_push_str("y")?;
    avm.avm1_op(0x1C);
    avm.avm1_op(0x0B); // Subtract
    avm.avm1_op(0x47);
    avm.avm1_op(0x26);

    // trace("prod=" + (6 * 12)) -> "prod=72"
    avm.avm1_push_str("prod=")?;
    avm.avm1_push_i32(6);
    avm.avm1_push_i32(12);
    avm.avm1_op(0x0C); // Multiply
    avm.avm1_op(0x47);
    avm.avm1_op(0x26);

    // trace("quot=" + (72 / 12)) -> "quot=6"
    avm.avm1_push_str("quot=")?;
    avm.avm1_push_i32(72);
    avm.avm1_push_i32(12);
    avm.avm1_op(0x0D); // Divide
    avm.avm1_op(0x47);
    avm.avm1_op(0x26);

    // trace("less=" + (12 < 30)) -> "less=true"
    avm.avm1_push_str("less=")?;
    avm.avm1_push_i32(12);
    avm.avm1_push_i32(30);
    avm.avm1_op(0x48); // Less2
    avm.avm1_op(0x47);
    avm.avm1_op(0x26);

    // trace("eq=" + (42 == 42)) -> "eq=true"
    avm.avm1_push_str("eq=")?;
    avm.avm1_push_i32(42);
    avm.avm1_push_i32(42);
    avm.avm1_op(0x49); // Equals2
    avm.avm1_op(0x47);
    avm.avm1_op(0x26);

    // DefineFunction & DefineFunction2 closures lifted & verified
    let mut fn_body = Avm1ActionBuilder::avm1_new();
    fn_body.avm1_push_i32(1);
    fn_body.avm1_op(0x3E); // Return
    let fn_bytes = fn_body.avm1_finish();
    avm.avm1_define_function("helper", &["a"], &fn_bytes)?;
    avm.avm1_define_function2("helper2", 2, 0, &[(1, "b")], &fn_bytes)?;

    avm.avm1_op(0x07); // Stop
    b.swf_builder_do_action(&avm.avm1_finish())?;
    b.swf_builder_show_frame()?;
    b.swf_builder_finish()
}

fn build_corpus_05_avm2_oop_swf() -> Result<Vec<u8>, CorpusError> {
    let mut abc = Vec::new();
    abc.extend_from_slice(&16u16.to_le_bytes()); // minor
    abc.extend_from_slice(&46u16.to_le_bytes()); // major

    // Constant pool: ints=0, uints=0, doubles=0
    abc.extend_from_slice(&[0, 0, 0]);
    // Strings (count=5 => 4 real entries: 1="Counter", 2="value", 3="increment", 4="Object")
    let strs = ["Counter", "value", "increment", "Object"];
    write_abc_u30(&mut abc, (strs.len() + 1) as u32)?;
    for s in strs {
        write_abc_u30(&mut abc, s.len() as u32)?;
        abc.extend_from_slice(s.as_bytes());
    }
    // Namespaces (count=2 => 1 entry: kind=0x16 PackageNamespace, name=0)
    abc.push(2);
    abc.push(0x16);
    abc.push(0);
    // NsSets (count=0)
    abc.push(0);
    // Multinames (count=5 => 4 QName entries: 1=Counter, 2=value, 3=increment, 4=Object)
    abc.push(5);
    for name_idx in 1u8..=4 {
        abc.push(0x07); // QName
        abc.push(1); // ns_idx = 1
        abc.push(name_idx);
    }

    // Methods (count=2): method 0 = script init, method 1 = Counter::increment
    abc.push(2);
    abc.extend_from_slice(&[0, 0, 0, 0]); // method 0
    abc.extend_from_slice(&[0, 0, 3, 0]); // method 1 (name="increment")

    // Metadata (count=0)
    abc.push(0);

    // Instances & Classes (count=1: class Counter extends Object)
    abc.push(1);
    abc.extend_from_slice(&[1, 4, 0, 0, 1, 0]); // instance: name=1, super=4, flags=0, ifaces=0, iinit=1, traits=0
    abc.extend_from_slice(&[0, 0]); // class: cinit=0, traits=0

    // Scripts (count=1: init_method=0, traits=0)
    abc.push(1);
    abc.extend_from_slice(&[0, 0]);

    // MethodBodies (count=2)
    abc.push(2);

    // Body 0 (method=0):
    // findpropstrict(1="Counter") [2]
    // constructprop(1="Counter", 0) [3]
    // setlocal_1 [1]
    // getlocal_1 [1]
    // pushbyte 40 [2]
    // setproperty(2="value") [2]
    // getlocal_1 [1]
    // callpropvoid(3="increment", 0) [3]
    // getlocal_1 [1]
    // getproperty(2="value") [2]
    // pushbyte 2 [2]
    // add [1]
    // returnvalue [1]
    let code0: Vec<u8> = vec![
        0x5D, 1, // findpropstrict Counter
        0x4A, 1, 0,    // constructprop Counter, 0
        0xD5, // setlocal_1
        0xD1, // getlocal_1
        0x24, 40, // pushbyte 40
        0x61, 2,    // setproperty value
        0xD1, // getlocal_1
        0x4F, 3, 0,    // callpropvoid increment, 0
        0xD1, // getlocal_1
        0x66, 2, // getproperty value
        0x24, 2,    // pushbyte 2
        0xA0, // add
        0x48, // returnvalue
    ];
    write_abc_u30(&mut abc, 0)?; // method_index
    write_abc_u30(&mut abc, 4)?; // max_stack
    write_abc_u30(&mut abc, 2)?; // local_count
    write_abc_u30(&mut abc, 0)?; // init_scope_depth
    write_abc_u30(&mut abc, 1)?; // max_scope_depth
    write_abc_u30(&mut abc, code0.len() as u32)?;
    abc.extend_from_slice(&code0);
    // 1 exception record (from=0, to=5, target=5, exc_type=1, var_name=2)
    abc.push(1);
    abc.extend_from_slice(&[0, 5, 5, 1, 2]);
    abc.push(0); // traits = 0

    // Body 1 (method=1): pushbyte 1, returnvalue
    let code1 = [0x24u8, 1, 0x48];
    write_abc_u30(&mut abc, 1)?;
    write_abc_u30(&mut abc, 2)?;
    write_abc_u30(&mut abc, 1)?;
    write_abc_u30(&mut abc, 0)?;
    write_abc_u30(&mut abc, 1)?;
    write_abc_u30(&mut abc, code1.len() as u32)?;
    abc.extend_from_slice(&code1);
    abc.push(0); // exceptions = 0
    abc.push(0); // traits = 0

    let mut b = SwfBuilder::swf_builder_new(9, SwfCompression::Fws, 320, 240, 24)?;
    b.swf_builder_define_shape(&rect_shape_spec(
        1,
        (400, 400, 1400, 1400),
        (0x00, 0x89, 0x7B),
    ))?;
    b.swf_builder_place_object(&PlaceTagSpec::place_new(1, 1))?;
    b.swf_builder_do_abc("corpus_05_oop", &abc)?;
    b.swf_builder_show_frame()?;
    b.swf_builder_finish()
}

fn build_corpus_06_streaming_audio_swf() -> Result<Vec<u8>, CorpusError> {
    let mut b = SwfBuilder::swf_builder_new(8, SwfCompression::Fws, 320, 240, 24)?;
    b.swf_builder_define_shape(&rect_shape_spec(1, (0, 0, 400, 400), (0xFF, 0xB3, 0x00)))?;

    // Event sound: 512 deterministic PCM16 samples
    let event_samples: Vec<i16> = (0..512)
        .map(|i| (((i * 313) % 4000) as i16).saturating_sub(2000))
        .collect();
    b.swf_builder_define_sound_pcm16(1, &event_samples)?;

    // Streaming audio: 4-bit ADPCM mono at 44100 Hz, 1838 samples/frame (~44100/24)
    let samples_per_frame = 1838usize;
    b.swf_builder_sound_stream_head2_adpcm(samples_per_frame as u16)?;

    let nibbles_a: Vec<u8> = (0..(samples_per_frame - 1))
        .map(|i| if i % 2 == 0 { 0x03 } else { 0x0B })
        .collect();
    let nibbles_b: Vec<u8> = (0..(samples_per_frame - 1))
        .map(|i| if i % 3 == 0 { 0x05 } else { 0x0D })
        .collect();
    let nibbles_c: Vec<u8> = (0..(samples_per_frame - 1))
        .map(|i| if i % 4 == 0 { 0x02 } else { 0x0A })
        .collect();

    // Frame 1: StartSound(1) + PlaceObject2 + ADPCM stream block
    b.swf_builder_start_sound(1)?;
    b.swf_builder_place_object(&PlaceTagSpec::place_new(1, 1))?;
    b.swf_builder_adpcm_stream_block(600, 20, &nibbles_a)?;
    b.swf_builder_show_frame()?;

    // Frame 2: Move shape + ADPCM stream block
    let mut m2 = PlaceTagSpec::place_move(1);
    m2.matrix = Some(SwfMatrix {
        translate_x: 600,
        translate_y: 200,
        ..SwfMatrix::default_matrix()
    });
    b.swf_builder_place_object(&m2)?;
    b.swf_builder_adpcm_stream_block(-400, 22, &nibbles_b)?;
    b.swf_builder_show_frame()?;

    // Frame 3: Move shape + ADPCM stream block
    let mut m3 = PlaceTagSpec::place_move(1);
    m3.matrix = Some(SwfMatrix {
        translate_x: 1200,
        translate_y: 400,
        ..SwfMatrix::default_matrix()
    });
    b.swf_builder_place_object(&m3)?;
    b.swf_builder_adpcm_stream_block(300, 18, &nibbles_c)?;
    b.swf_builder_show_frame()?;

    b.swf_builder_finish()
}

/// Builds a compact RFC 1950 + fixed-Huffman DEFLATE `CWS` bomb inflating to `target_bytes`.
fn build_cws_decompression_bomb_swf(target_bytes: usize) -> Result<Vec<u8>, CorpusError> {
    struct LsbWriter {
        bytes: Vec<u8>,
        cur: u8,
        used: u8,
    }
    impl LsbWriter {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                cur: 0,
                used: 0,
            }
        }
        fn push_lsb(&mut self, val: u32, bits: u8) {
            for i in 0..bits {
                let bit = ((val >> i) & 1) as u8;
                self.cur |= bit << self.used;
                self.used += 1;
                if self.used == 8 {
                    self.bytes.push(self.cur);
                    self.cur = 0;
                    self.used = 0;
                }
            }
        }
        fn push_msb(&mut self, val: u32, bits: u8) {
            for i in (0..bits).rev() {
                let bit = ((val >> i) & 1) as u8;
                self.cur |= bit << self.used;
                self.used += 1;
                if self.used == 8 {
                    self.bytes.push(self.cur);
                    self.cur = 0;
                    self.used = 0;
                }
            }
        }
        fn finish(mut self) -> Vec<u8> {
            if self.used > 0 {
                self.bytes.push(self.cur);
            }
            self.bytes
        }
    }

    let mut lw = LsbWriter::new();
    // BFINAL=1 (1 bit), BTYPE=01 fixed Huffman (2 bits LSB-first)
    lw.push_lsb(1, 1);
    lw.push_lsb(1, 2);
    // Literal 0x00 (8-bit canonical code 0b00110000 = 0x30)
    lw.push_msb(0x30, 8);
    // Repeat 258-byte back-reference at distance 1 until > target_bytes
    let repeats = target_bytes.div_ceil(258);
    for _ in 0..repeats {
        // Symbol 285 (8-bit canonical code 0xC5), 0 extra length bits
        lw.push_msb(0xC5, 8);
        // Distance code 0 (5-bit code 00000), 0 extra dist bits
        lw.push_msb(0, 5);
    }
    // Symbol 256 end-of-block (7-bit code 0000000)
    lw.push_msb(0, 7);
    let deflate_stream = lw.finish();

    let mut out = Vec::with_capacity(8 + 2 + deflate_stream.len() + 4);
    out.extend_from_slice(b"CWS");
    out.push(8);
    out.extend_from_slice(&(target_bytes as u32).to_le_bytes());
    out.extend_from_slice(&[0x78, 0x9C]);
    out.extend_from_slice(&deflate_stream);
    out.extend_from_slice(&[0u8; 4]);
    Ok(out)
}
