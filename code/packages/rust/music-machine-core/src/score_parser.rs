//! Iterative parser for beginner-friendly sheet music (MUS01 / MUS03).

use crate::error::MusicMachineError;
use crate::limits::*;
use crate::types::*;

const DIRECTIVE_NAMES: &[&str] = &[
    "title",
    "tempo",
    "meter",
    "amplitude",
    "sample_rate",
    "instrument",
    "program",
    "drumkit",
];

/// Linear-time scanner for directive lines matching `name: value` (CWE-1333).
fn parse_directive_line(line: &str) -> Option<(&str, &str)> {
    let colon_idx = line.find(':')?;
    let name = line[..colon_idx].trim();
    if name.is_empty() {
        return None;
    }
    let first = name.as_bytes()[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    for &ch in &name.as_bytes()[1..] {
        if !(ch.is_ascii_alphanumeric() || ch == b'_') {
            return None;
        }
    }
    let value = line[colon_idx + 1..].trim();
    Some((name, value))
}

/// Parse a single music token (e.g. "G4/e", "R/q", "kick/s") into a `ScoreEvent`.
fn parse_score_token(
    token: &str,
    tempo_bpm: f64,
    is_drum_score: bool,
) -> Result<ScoreEvent, MusicMachineError> {
    let parts: Vec<&str> = token.split('/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(MusicMachineError::ParseError(format!(
            "music token '{token}' must look like pitch/duration"
        )));
    }
    let pitch_text = parts[0];
    let duration_symbol = parts[1];

    let beat_count = beats_for_duration_symbol(duration_symbol)?;
    let seconds_per_beat = 60.0 / tempo_bpm;
    let duration_seconds = beat_count * seconds_per_beat;

    if pitch_text.eq_ignore_ascii_case("r") || pitch_text.eq_ignore_ascii_case("rest") {
        return ScoreEvent::new(
            ScoreEventKind::Rest,
            None,
            None,
            duration_symbol.to_string(),
            beat_count,
            duration_seconds,
            token.to_string(),
        );
    }

    let pitch_lower = pitch_text.to_ascii_lowercase();
    if is_drum_score && percussion_kit::ALL_STANDARD_HIT_IDS.contains(&pitch_lower.as_str()) {
        return ScoreEvent::new(
            ScoreEventKind::Hit,
            None,
            Some(pitch_lower),
            duration_symbol.to_string(),
            beat_count,
            duration_seconds,
            token.to_string(),
        );
    }

    // Try parsing as a note
    let parsed_note = musical_instruments::parse_note(pitch_text).map_err(|e| {
        MusicMachineError::ParseError(format!("invalid note spelling '{pitch_text}': {e}"))
    })?;

    ScoreEvent::new(
        ScoreEventKind::Note,
        Some(parsed_note.to_string()),
        None,
        duration_symbol.to_string(),
        beat_count,
        duration_seconds,
        token.to_string(),
    )
}

/// Parse beginner-friendly text sheet music into a `TextScore`.
pub fn parse_score(text: &str) -> Result<TextScore, MusicMachineError> {
    parse_score_with_limits(
        text,
        DEFAULT_MAX_SCORE_LENGTH,
        DEFAULT_MAX_LINE_LENGTH,
        DEFAULT_MAX_EVENT_COUNT,
    )
}

/// Parse beginner-friendly text sheet music with explicit resource bounds (CWE-400 / CWE-674).
pub fn parse_score_with_limits(
    text: &str,
    max_score_length: usize,
    max_line_length: usize,
    max_event_count: usize,
) -> Result<TextScore, MusicMachineError> {
    if text.len() > max_score_length {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "score text length {} exceeds max_score_length={max_score_length}",
            text.len()
        )));
    }

    let mut title = "Untitled".to_string();
    let mut tempo_bpm = DEFAULT_TEMPO_BPM;
    let mut meter = DEFAULT_METER.to_string();
    let mut amplitude = DEFAULT_AMPLITUDE;
    let mut sample_rate_hz = DEFAULT_SAMPLE_RATE_HZ;
    let mut instrument_id = DEFAULT_INSTRUMENT_ID.to_string();
    let mut instrument_directive_seen = false;
    let mut gm_program: Option<u8> = None;
    let mut drumkit: Option<String> = None;
    let mut events = Vec::new();

    for (line_idx, raw_line) in text.lines().enumerate() {
        if line_idx >= MAX_LINES {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "line count exceeds MAX_LINES={MAX_LINES}"
            )));
        }
        let line_number = line_idx + 1;
        if raw_line.len() > max_line_length {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "line {line_number}: line length {} exceeds max_line_length={max_line_length}",
                raw_line.len()
            )));
        }

        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((name, value)) = parse_directive_line(line) {
            if !events.is_empty() {
                return Err(MusicMachineError::ParseError(format!(
                    "line {line_number}: directive '{name}' appears after music"
                )));
            }
            if !DIRECTIVE_NAMES.contains(&name) {
                return Err(MusicMachineError::ParseError(format!(
                    "line {line_number}: unknown directive '{name}'"
                )));
            }
            if value.is_empty() {
                return Err(MusicMachineError::ParseError(format!(
                    "line {line_number}: directive '{name}' is empty"
                )));
            }

            match name {
                "title" => {
                    title = value.to_string();
                }
                "tempo" => {
                    let val: f64 = value.parse().map_err(|_| {
                        MusicMachineError::ParseError(format!(
                            "line {line_number}: tempo must be a number, got '{value}'"
                        ))
                    })?;
                    validate_positive("tempo", val)?;
                    tempo_bpm = val;
                }
                "meter" => {
                    validate_meter(value).map_err(|e| {
                        MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                    })?;
                    meter = value.to_string();
                }
                "amplitude" => {
                    let val: f64 = value.parse().map_err(|_| {
                        MusicMachineError::ParseError(format!(
                            "line {line_number}: amplitude must be a number, got '{value}'"
                        ))
                    })?;
                    validate_unit_float("amplitude", val).map_err(|e| {
                        MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                    })?;
                    amplitude = val;
                }
                "sample_rate" => {
                    let val: f64 = value.parse().map_err(|_| {
                        MusicMachineError::ParseError(format!(
                            "line {line_number}: sample_rate must be an integer, got '{value}'"
                        ))
                    })?;
                    if !val.is_finite() || val.fract() != 0.0 || val <= 0.0 {
                        return Err(MusicMachineError::ParseError(format!(
                            "line {line_number}: sample_rate must be a positive integer, got '{value}'"
                        )));
                    }
                    if val > MAX_SAMPLE_RATE_HZ {
                        return Err(MusicMachineError::BudgetExceeded(format!(
                            "line {line_number}: sample_rate {val} exceeds MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
                        )));
                    }
                    sample_rate_hz = val as u32;
                }
                "instrument" => {
                    if gm_program.is_some() {
                        return Err(MusicMachineError::ParseError(format!(
                            "line {line_number}: score may specify instrument or program, not both"
                        )));
                    }
                    if value == "drumkit" || value == "standard_kit" || value == "standard" {
                        instrument_id = "drumkit".to_string();
                        drumkit = Some("standard_kit".to_string());
                    } else {
                        let resolved = resolve_instrument_id(Some(value), None).map_err(|e| {
                            MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                        })?;
                        instrument_id = resolved;
                    }
                    instrument_directive_seen = true;
                }
                "program" => {
                    if instrument_directive_seen {
                        return Err(MusicMachineError::ParseError(format!(
                            "line {line_number}: score may specify instrument or program, not both"
                        )));
                    }
                    let prog: u8 = value.parse().map_err(|_| {
                        MusicMachineError::ParseError(format!(
                            "line {line_number}: program must be an integer in [1, 128], got '{value}'"
                        ))
                    })?;
                    if !(1..=128).contains(&prog) {
                        return Err(MusicMachineError::ParseError(format!(
                            "line {line_number}: program must be in [1, 128], got {prog}"
                        )));
                    }
                    gm_program = Some(prog);
                    let resolved = resolve_instrument_id(None, Some(prog)).map_err(|e| {
                        MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                    })?;
                    instrument_id = resolved;
                }
                "drumkit" => {
                    drumkit = Some(value.to_string());
                    instrument_id = "drumkit".to_string();
                }
                _ => unreachable!(),
            }
            continue;
        }

        // Parse music tokens on event lines
        let is_drums = instrument_id == "drumkit" || drumkit.is_some();
        for token in line.split_whitespace() {
            if token == "|" {
                continue;
            }
            if events.len() >= max_event_count {
                return Err(MusicMachineError::BudgetExceeded(format!(
                    "line {line_number}: event count exceeds max_event_count={max_event_count}"
                )));
            }
            let ev = parse_score_token(token, tempo_bpm, is_drums).map_err(|e| {
                MusicMachineError::ParseError(format!("line {line_number}: {e}"))
            })?;
            events.push(ev);
        }
    }

    if events.is_empty() {
        return Err(MusicMachineError::InvalidArgument(
            "score must contain at least one event".into(),
        ));
    }

    Ok(TextScore {
        title,
        tempo_bpm,
        meter,
        amplitude,
        sample_rate_hz,
        instrument_id,
        gm_program,
        events,
        drumkit,
    })
}

fn validate_positive(name: &str, val: f64) -> Result<(), MusicMachineError> {
    if !val.is_finite() || val <= 0.0 {
        return Err(MusicMachineError::InvalidArgument(format!(
            "{name} must be > 0.0, got {val}"
        )));
    }
    Ok(())
}
