//! # Music Machine Core (MUS01 / MUS03 / MUS04 / MUS07 / VER04)
//!
//! Pure Rust deterministic sheet music parser, multi-track synthesizer, and WAV generator.
//!
//! ## Verification & Security Guarantees
//!
//! - **Safety**: `#![forbid(unsafe_code)]`, zero external third-party dependencies.
//! - **CWE-400 (Resource Exhaustion)**: Strict pre-flight budget verification enforcing bounds
//!   on score bytes, lines, events, duration, sample rate, partials, and JSON nesting depth.
//! - **CWE-190 (Integer Overflow)**: Checked arithmetic on sample counts, timeline indices, ticks, and byte lengths.
//! - **CWE-20 (Improper Input Validation)**: Finite float checks rejecting `NaN` and `±Inf`.
//! - **CWE-674 (Uncontrolled Recursion)**: Strictly iterative tokenizers and parsers with explicit depth guards on JSON (`MAX_JSON_DEPTH = 32`).
//! - **CWE-1333 (ReDoS)**: Zero regular expressions across all parsing paths.

#![forbid(unsafe_code)]

pub mod error;
pub mod io;
pub mod json;
pub mod limits;
pub mod portable_parser;
pub mod render;
pub mod score_parser;
pub mod tempo;
pub mod types;

// Re-exports
pub use error::MusicMachineError;
pub use io::{
    parse_portable_score_file, parse_score_file, render_portable_score_to_wav_file,
    render_score_to_wav_file,
};
pub use json::{escape_json_string, format_json_float, parse_json, JsonValue};
pub use limits::{
    DEFAULT_AMPLITUDE, DEFAULT_INSTRUMENT_ID, DEFAULT_MAX_EVENT_COUNT, DEFAULT_MAX_LINE_LENGTH,
    DEFAULT_MAX_SAMPLE_COUNT, DEFAULT_MAX_SCORE_LENGTH, DEFAULT_METER, DEFAULT_PPQ,
    DEFAULT_SAMPLE_RATE_HZ, DEFAULT_TEMPO_BPM, HAPPY_BIRTHDAY_TEXT, MAX_DRUM_HITS,
    MAX_DURATION_SECONDS, MAX_EVENTS, MAX_INSTRUMENTS, MAX_JSON_DEPTH, MAX_LINES,
    MAX_LINE_LENGTH, MAX_PARTIALS, MAX_PPQ, MAX_SAMPLE_COUNT, MAX_SAMPLE_RATE_HZ,
    MAX_SCORE_BYTES, MAX_SCORE_LENGTH, MAX_TRACKS, MIN_SAMPLE_RATE_HZ, PORTABLE_FORMAT_VERSION,
};
pub use portable_parser::{parse_portable_score, parse_portable_score_with_limits};

/// Parse a portable multi-track JSON score definition.
pub fn parse_portable_score_json(json_text: &str) -> Result<PortableScore, MusicMachineError> {
    PortableScore::from_json(json_text)
}
pub use render::{mix_pcm_samples, render_portable_score_to_pcm, render_score_to_pcm};
pub use score_parser::{parse_score, parse_score_with_limits};
pub use tempo::TempoMap;
pub use types::{
    beats_for_duration_symbol, is_identifier, is_valid_meter, resolve_instrument_id,
    validate_finite, validate_identifier, validate_meter, validate_unit_float,
    InstrumentDeclaration, MeterEvent, PortableScore, PortableScoreEvent, RenderedPortableScore,
    RenderedScore, ScoreEvent, ScoreEventKind, TempoEvent, TextScore, TrackDeclaration,
};
pub use musical_instruments::HarmonicPartial;

// ============================================================================
// Warrant Operations (matching music-machine-core.warrant specifications)
// ============================================================================

/// Model/operation for warrant specification: verify score byte length budget.
pub fn warrant_parse_score(score_bytes: u64) -> Result<u64, MusicMachineError> {
    if score_bytes > (MAX_SCORE_BYTES as u64) {
        return Err(MusicMachineError::BudgetExceeded(
            "score_bytes > MAX_SCORE_BYTES".into(),
        ));
    }
    Ok(score_bytes)
}

/// Model/operation for warrant specification: verify portable score byte length budget.
pub fn warrant_parse_portable_score(score_bytes: u64) -> Result<u64, MusicMachineError> {
    if score_bytes > (MAX_SCORE_BYTES as u64) {
        return Err(MusicMachineError::BudgetExceeded(
            "score_bytes > MAX_SCORE_BYTES".into(),
        ));
    }
    Ok(score_bytes)
}

/// Model/operation for warrant specification: verify render sample rate and event count budget.
pub fn warrant_render_score_to_pcm(
    event_count: u64,
    sample_rate_hz: u64,
) -> Result<u64, MusicMachineError> {
    if sample_rate_hz == 0 || sample_rate_hz > (MAX_SAMPLE_RATE_HZ as u64) {
        return Err(MusicMachineError::InvalidArgument(
            "invalid sample_rate_hz".into(),
        ));
    }
    if event_count > (MAX_EVENTS as u64) {
        return Err(MusicMachineError::BudgetExceeded(
            "event_count > MAX_EVENTS".into(),
        ));
    }
    Ok(event_count)
}

/// Model/operation for warrant specification: verify portable render sample rate and event count budget.
pub fn warrant_render_portable_score_to_pcm(
    event_count: u64,
    sample_rate_hz: u64,
) -> Result<u64, MusicMachineError> {
    if sample_rate_hz == 0 || sample_rate_hz > (MAX_SAMPLE_RATE_HZ as u64) {
        return Err(MusicMachineError::InvalidArgument(
            "invalid sample_rate_hz".into(),
        ));
    }
    if event_count > (MAX_EVENTS as u64) {
        return Err(MusicMachineError::BudgetExceeded(
            "event_count > MAX_EVENTS".into(),
        ));
    }
    Ok(event_count)
}
