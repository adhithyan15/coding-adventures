//! Iterative parser and formatter for canonical `music-machine-score/v2` portable scores (MUS04).

use std::collections::HashMap;

use musical_instruments::{
    instrument_for_gm_program, AdsrEnvelope, HarmonicPartial, MAX_NOTE_DURATION_SECONDS,
};

use crate::error::MusicMachineError;
use crate::json::format_json_float;
use crate::limits::*;
use crate::types::*;

/// Parse property tokens matching `key=value` (linear-time, zero regex).
fn parse_properties<'a>(tokens: &'a [&'a str]) -> Result<HashMap<&'a str, &'a str>, MusicMachineError> {
    let mut props = HashMap::new();
    for &tok in tokens {
        let parts: Vec<&str> = tok.split('=').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(MusicMachineError::ParseError(format!(
                "property '{tok}' must be key=value"
            )));
        }
        let key = parts[0];
        let val = parts[1];
        if props.insert(key, val).is_some() {
            return Err(MusicMachineError::ParseError(format!(
                "duplicate property '{key}'"
            )));
        }
    }
    Ok(props)
}

/// Parse comma-separated harmonic partials e.g. "1:1.0,2:0.5".
fn parse_harmonics(text: &str) -> Result<Vec<HarmonicPartial>, MusicMachineError> {
    let items: Vec<&str> = text.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if items.is_empty() {
        return Err(MusicMachineError::InvalidArgument(
            "harmonics must contain at least one partial".into(),
        ));
    }
    if items.len() > MAX_PARTIALS {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "harmonics count {} exceeds MAX_PARTIALS={MAX_PARTIALS}",
            items.len()
        )));
    }

    let mut partials = Vec::with_capacity(items.len());
    for item in items {
        let parts: Vec<&str> = item.split(':').collect();
        if parts.len() != 2 && parts.len() != 3 {
            return Err(MusicMachineError::ParseError(format!(
                "harmonic partial '{item}' must be multiplier:amplitude[:phase]"
            )));
        }
        let mult: f64 = parts[0].parse().map_err(|_| {
            MusicMachineError::ParseError(format!("invalid multiplier in partial '{item}'"))
        })?;
        let amp: f64 = parts[1].parse().map_err(|_| {
            MusicMachineError::ParseError(format!("invalid amplitude in partial '{item}'"))
        })?;
        let phase: f64 = if parts.len() == 3 {
            parts[2].parse().map_err(|_| {
                MusicMachineError::ParseError(format!("invalid phase in partial '{item}'"))
            })?
        } else {
            0.0
        };
        partials.push(HarmonicPartial::with_phase(mult, amp, phase)?);
    }
    Ok(partials)
}

/// Parse comma-separated ADSR envelope "attack,decay,sustain,release".
fn parse_envelope(text: &str) -> Result<AdsrEnvelope, MusicMachineError> {
    let parts: Vec<&str> = text.split(',').map(|s| s.trim()).collect();
    if parts.len() != 4 {
        return Err(MusicMachineError::ParseError(format!(
            "envelope must be attack,decay,sustain,release, got '{text}'"
        )));
    }
    let atk: f64 = parts[0].parse().map_err(|_| {
        MusicMachineError::ParseError(format!("invalid attack float '{}'", parts[0]))
    })?;
    let dec: f64 = parts[1].parse().map_err(|_| {
        MusicMachineError::ParseError(format!("invalid decay float '{}'", parts[1]))
    })?;
    let sus: f64 = parts[2].parse().map_err(|_| {
        MusicMachineError::ParseError(format!("invalid sustain float '{}'", parts[2]))
    })?;
    let rel: f64 = parts[3].parse().map_err(|_| {
        MusicMachineError::ParseError(format!("invalid release float '{}'", parts[3]))
    })?;

    if atk > MAX_NOTE_DURATION_SECONDS || dec > MAX_NOTE_DURATION_SECONDS || rel > MAX_NOTE_DURATION_SECONDS {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "envelope times must be <= {MAX_NOTE_DURATION_SECONDS}"
        )));
    }

    Ok(AdsrEnvelope::new(atk, dec, sus, rel)?)
}

/// Parse canonical `music-machine-score/v2` multi-track score text.
pub fn parse_portable_score(text: &str) -> Result<PortableScore, MusicMachineError> {
    parse_portable_score_with_limits(
        text,
        DEFAULT_MAX_SCORE_LENGTH,
        DEFAULT_MAX_LINE_LENGTH,
        DEFAULT_MAX_EVENT_COUNT,
    )
}

/// Parse canonical `music-machine-score/v2` multi-track score text with explicit bounds.
pub fn parse_portable_score_with_limits(
    text: &str,
    max_score_length: usize,
    max_line_length: usize,
    max_event_count: usize,
) -> Result<PortableScore, MusicMachineError> {
    if text.len() > max_score_length {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "score text length {} exceeds max_score_length={max_score_length}",
            text.len()
        )));
    }

    let mut saw_format = false;
    let mut title = "Untitled".to_string();
    let mut ppq = DEFAULT_PPQ;
    let mut sample_rate_hz = DEFAULT_SAMPLE_RATE_HZ;
    let mut tempo_events = Vec::new();
    let mut meter_events = Vec::new();
    let mut instruments = Vec::new();
    let mut tracks = Vec::new();
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

        if !saw_format {
            if line != format!("format: {PORTABLE_FORMAT_VERSION}") {
                return Err(MusicMachineError::ParseError(format!(
                    "line {line_number}: first directive must be 'format: {PORTABLE_FORMAT_VERSION}'"
                )));
            }
            saw_format = true;
            continue;
        }

        // Directives: title:, ppq:, sample_rate: (matching ^[a-z_]+:\s*(.*)$)
        if let Some(colon_idx) = line.find(':') {
            let prefix = &line[..colon_idx];
            if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                let name = prefix;
                let value = line[colon_idx + 1..].trim();
                match name {
                    "title" => {
                        title = value.to_string();
                    }
                    "ppq" => {
                        let val: u32 = value.parse().map_err(|_| {
                            MusicMachineError::ParseError(format!(
                                "line {line_number}: ppq must be an integer, got '{value}'"
                            ))
                        })?;
                        if val == 0 || val > MAX_PPQ {
                            return Err(MusicMachineError::InvalidArgument(format!(
                                "line {line_number}: ppq must be in [1, {MAX_PPQ}], got {val}"
                            )));
                        }
                        ppq = val;
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
                    _ => {
                        return Err(MusicMachineError::ParseError(format!(
                            "line {line_number}: unknown directive '{name}'"
                        )));
                    }
                }
                continue;
            }
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let command = tokens[0];

        match command {
            "tempo" => {
                if tokens.len() != 3 {
                    return Err(MusicMachineError::ParseError(format!(
                        "line {line_number}: tempo lines must be: tempo <start_tick> <bpm>"
                    )));
                }
                let start_tick: u64 = tokens[1].parse().map_err(|_| {
                    MusicMachineError::ParseError(format!(
                        "line {line_number}: invalid tempo start_tick '{}'",
                        tokens[1]
                    ))
                })?;
                let bpm: f64 = tokens[2].parse().map_err(|_| {
                    MusicMachineError::ParseError(format!(
                        "line {line_number}: invalid tempo bpm '{}'",
                        tokens[2]
                    ))
                })?;
                if !bpm.is_finite() || bpm <= 0.0 {
                    return Err(MusicMachineError::InvalidArgument(format!(
                        "line {line_number}: tempo bpm must be > 0.0, got {bpm}"
                    )));
                }
                tempo_events.push(TempoEvent { start_tick, bpm });
            }
            "meter" => {
                if tokens.len() != 3 {
                    return Err(MusicMachineError::ParseError(format!(
                        "line {line_number}: meter lines must be: meter <start_tick> <meter>"
                    )));
                }
                let start_tick: u64 = tokens[1].parse().map_err(|_| {
                    MusicMachineError::ParseError(format!(
                        "line {line_number}: invalid meter start_tick '{}'",
                        tokens[1]
                    ))
                })?;
                validate_meter(tokens[2]).map_err(|e| {
                    MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                })?;
                meter_events.push(MeterEvent {
                    start_tick,
                    meter: tokens[2].to_string(),
                });
            }
            "instrument" => {
                if tokens.len() < 3 {
                    return Err(MusicMachineError::ParseError(format!(
                        "line {line_number}: instrument lines must include id and properties"
                    )));
                }
                let inst_id = tokens[1];
                validate_identifier("instrument id", inst_id).map_err(|e| {
                    MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                })?;

                let props = parse_properties(&tokens[2..]).map_err(|e| {
                    MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                })?;

                let allowed = ["gain", "program", "profile", "kind", "kit", "harmonics", "envelope"];
                for k in props.keys() {
                    if !allowed.contains(k) {
                        return Err(MusicMachineError::ParseError(format!(
                            "line {line_number}: unknown instrument property '{k}'"
                        )));
                    }
                }

                let gain = if let Some(&g_str) = props.get("gain") {
                    let g: f64 = g_str.parse().map_err(|_| {
                        MusicMachineError::ParseError(format!(
                            "line {line_number}: invalid gain float '{g_str}'"
                        ))
                    })?;
                    validate_unit_float("gain", g).map_err(|e| {
                        MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                    })?;
                    g
                } else {
                    1.0
                };

                let selector_count = props.contains_key("program") as usize
                    + props.contains_key("profile") as usize
                    + props.contains_key("kind") as usize;
                if selector_count != 1 {
                    return Err(MusicMachineError::ParseError(format!(
                        "line {line_number}: instrument must specify exactly one of program, profile, or kind"
                    )));
                }

                if let Some(&prog_str) = props.get("program") {
                    let prog: u8 = prog_str.parse().map_err(|_| {
                        MusicMachineError::ParseError(format!(
                            "line {line_number}: invalid program '{prog_str}'"
                        ))
                    })?;
                    if !(1..=128).contains(&prog) {
                        return Err(MusicMachineError::ParseError(format!(
                            "line {line_number}: program must be in [1, 128], got {prog}"
                        )));
                    }
                    let p = instrument_for_gm_program(prog)?;
                    instruments.push(InstrumentDeclaration::new(
                        inst_id,
                        &p.id,
                        gain,
                        "preset",
                        None,
                        None,
                        None,
                    )?);
                } else if let Some(&prof_str) = props.get("profile") {
                    let p_id = resolve_instrument_id(Some(prof_str), None).map_err(|e| {
                        MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                    })?;
                    instruments.push(InstrumentDeclaration::new(
                        inst_id,
                        &p_id,
                        gain,
                        "preset",
                        None,
                        None,
                        None,
                    )?);
                } else if let Some(&kind_str) = props.get("kind") {
                    match kind_str {
                        "sine" | "silence" => {
                            instruments.push(InstrumentDeclaration::new(
                                inst_id,
                                kind_str,
                                gain,
                                kind_str,
                                None,
                                None,
                                None,
                            )?);
                        }
                        "drumkit" => {
                            let kit_name = props.get("kit").copied().unwrap_or("standard_kit");
                            let norm_kit = if kit_name == "standard" {
                                "standard_kit"
                            } else {
                                kit_name
                            };
                            instruments.push(InstrumentDeclaration::new(
                                inst_id,
                                "drumkit",
                                gain,
                                "drumkit",
                                Some(norm_kit.to_string()),
                                None,
                                None,
                            )?);
                        }
                        "additive" => {
                            let harm_str = props.get("harmonics").ok_or_else(|| {
                                MusicMachineError::ParseError(format!(
                                    "line {line_number}: kind=additive must provide harmonics"
                                ))
                            })?;
                            let harmonics = parse_harmonics(harm_str).map_err(|e| match e {
                                MusicMachineError::BudgetExceeded(_) => e,
                                other => MusicMachineError::ParseError(format!("line {line_number}: {other}")),
                            })?;
                            let envelope = if let Some(&env_str) = props.get("envelope") {
                                Some(parse_envelope(env_str).map_err(|e| match e {
                                    MusicMachineError::BudgetExceeded(_) => e,
                                    other => MusicMachineError::ParseError(format!("line {line_number}: {other}")),
                                })?)
                            } else {
                                None
                            };
                            instruments.push(InstrumentDeclaration::new(
                                inst_id,
                                inst_id,
                                gain,
                                "additive",
                                None,
                                Some(harmonics),
                                envelope,
                            )?);
                        }
                        _ => {
                            return Err(MusicMachineError::ParseError(format!(
                                "line {line_number}: unsupported instrument kind '{kind_str}'"
                            )));
                        }
                    }
                }
            }
            "track" => {
                if tokens.len() < 3 {
                    return Err(MusicMachineError::ParseError(format!(
                        "line {line_number}: track lines must include id and instrument=<id>"
                    )));
                }
                let track_id = tokens[1];
                validate_identifier("track id", track_id).map_err(|e| {
                    MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                })?;

                let props = parse_properties(&tokens[2..]).map_err(|e| {
                    MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                })?;

                let inst_id = props.get("instrument").ok_or_else(|| {
                    MusicMachineError::ParseError(format!(
                        "line {line_number}: track must include instrument=<id>"
                    ))
                })?;

                for k in props.keys() {
                    if *k != "instrument" {
                        return Err(MusicMachineError::ParseError(format!(
                            "line {line_number}: unknown track property '{k}'"
                        )));
                    }
                }

                tracks.push(TrackDeclaration::new(track_id, inst_id)?);
            }
            "event" => {
                if events.len() >= max_event_count {
                    return Err(MusicMachineError::BudgetExceeded(format!(
                        "line {line_number}: event count exceeds max_event_count={max_event_count}"
                    )));
                }
                if tokens.len() < 5 {
                    return Err(MusicMachineError::ParseError(format!(
                        "line {line_number}: event lines must be: event <track> <start> <duration> note|rest|hit ..."
                    )));
                }
                let track_id = tokens[1];
                let start_tick: u64 = tokens[2].parse().map_err(|_| {
                    MusicMachineError::ParseError(format!(
                        "line {line_number}: invalid event start_tick '{}'",
                        tokens[2]
                    ))
                })?;
                let duration_tick: u64 = tokens[3].parse().map_err(|_| {
                    MusicMachineError::ParseError(format!(
                        "line {line_number}: invalid event duration_tick '{}'",
                        tokens[3]
                    ))
                })?;
                let kind_str = tokens[4];
                let kind = ScoreEventKind::from_str_strict(kind_str).map_err(|e| {
                    MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                })?;

                let source_order = events.len();

                match kind {
                    ScoreEventKind::Rest => {
                        if tokens.len() != 5 {
                            return Err(MusicMachineError::ParseError(format!(
                                "line {line_number}: rest events must not include notes or properties"
                            )));
                        }
                        events.push(PortableScoreEvent::new(
                            track_id,
                            start_tick,
                            duration_tick,
                            ScoreEventKind::Rest,
                            Vec::new(),
                            1.0,
                            source_order,
                            None,
                        )?);
                    }
                    ScoreEventKind::Hit => {
                        if tokens.len() < 6 {
                            return Err(MusicMachineError::ParseError(format!(
                                "line {line_number}: hit events must include a hit_id"
                            )));
                        }
                        let hit_id = tokens[5].to_ascii_lowercase();
                        let props = parse_properties(&tokens[6..]).map_err(|e| {
                            MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                        })?;
                        for k in props.keys() {
                            if *k != "velocity" {
                                return Err(MusicMachineError::ParseError(format!(
                                    "line {line_number}: unknown event property '{k}'"
                                )));
                            }
                        }
                        let velocity = if let Some(&v_str) = props.get("velocity") {
                            let v: f64 = v_str.parse().map_err(|_| {
                                MusicMachineError::ParseError(format!(
                                    "line {line_number}: invalid velocity float '{v_str}'"
                                ))
                            })?;
                            validate_unit_float("velocity", v).map_err(|e| {
                                MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                            })?;
                            v
                        } else {
                            1.0
                        };

                        events.push(PortableScoreEvent::new(
                            track_id,
                            start_tick,
                            duration_tick,
                            ScoreEventKind::Hit,
                            Vec::new(),
                            velocity,
                            source_order,
                            Some(hit_id),
                        )?);
                    }
                    ScoreEventKind::Note => {
                        if tokens.len() < 6 {
                            return Err(MusicMachineError::ParseError(format!(
                                "line {line_number}: note events must include a pitch list"
                            )));
                        }
                        let pitch_list = tokens[5];
                        let note_strings: Vec<String> = pitch_list
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        if note_strings.is_empty() {
                            return Err(MusicMachineError::ParseError(format!(
                                "line {line_number}: note events must include at least one note"
                            )));
                        }

                        let props = parse_properties(&tokens[6..]).map_err(|e| {
                            MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                        })?;
                        for k in props.keys() {
                            if *k != "velocity" {
                                return Err(MusicMachineError::ParseError(format!(
                                    "line {line_number}: unknown event property '{k}'"
                                )));
                            }
                        }
                        let velocity = if let Some(&v_str) = props.get("velocity") {
                            let v: f64 = v_str.parse().map_err(|_| {
                                MusicMachineError::ParseError(format!(
                                    "line {line_number}: invalid velocity float '{v_str}'"
                                ))
                            })?;
                            validate_unit_float("velocity", v).map_err(|e| {
                                MusicMachineError::ParseError(format!("line {line_number}: {e}"))
                            })?;
                            v
                        } else {
                            1.0
                        };

                        events.push(PortableScoreEvent::new(
                            track_id,
                            start_tick,
                            duration_tick,
                            ScoreEventKind::Note,
                            note_strings,
                            velocity,
                            source_order,
                            None,
                        )?);
                    }
                }
            }
            _ => {
                return Err(MusicMachineError::ParseError(format!(
                    "line {line_number}: unknown command '{command}'"
                )));
            }
        }
    }

    if !saw_format {
        return Err(MusicMachineError::ParseError(format!(
            "first directive must be 'format: {PORTABLE_FORMAT_VERSION}'"
        )));
    }

    PortableScore::new(
        PORTABLE_FORMAT_VERSION,
        &title,
        ppq,
        sample_rate_hz,
        tempo_events,
        meter_events,
        instruments,
        tracks,
        events,
    )
}

impl PortableScore {
    /// Format this portable score back to canonical human-readable V2 text.
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("format: {}", self.format_version));
        lines.push(format!("title: {}", self.title));
        lines.push(format!("ppq: {}", self.ppq));
        lines.push(format!("sample_rate: {}", self.sample_rate_hz));

        for t in &self.tempo_events {
            lines.push(format!("tempo {} {}", t.start_tick, format_json_float(t.bpm)));
        }
        for m in &self.meter_events {
            lines.push(format!("meter {} {}", m.start_tick, m.meter));
        }

        if !self.instruments.is_empty() {
            lines.push(String::new());
            for inst in &self.instruments {
                match inst.kind.as_str() {
                    "drumkit" => {
                        let kit = inst.kit.as_deref().unwrap_or("standard_kit");
                        lines.push(format!(
                            "instrument {} kind=drumkit kit={} gain={}",
                            inst.id,
                            kit,
                            format_json_float(inst.gain)
                        ));
                    }
                    "additive" => {
                        let harmonics_str = if let Some(ref harms) = inst.harmonics {
                            harms
                                .iter()
                                .map(|h| {
                                    if h.phase_cycles != 0.0 {
                                        format!(
                                            "{}:{}:{}",
                                            format_json_float(h.frequency_multiplier),
                                            format_json_float(h.amplitude),
                                            format_json_float(h.phase_cycles)
                                        )
                                    } else {
                                        format!(
                                            "{}:{}",
                                            format_json_float(h.frequency_multiplier),
                                            format_json_float(h.amplitude)
                                        )
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(",")
                        } else {
                            String::new()
                        };

                        let env_str = if let Some(ref env) = inst.envelope {
                            format!(
                                "{},{},{},{}",
                                format_json_float(env.attack_seconds),
                                format_json_float(env.decay_seconds),
                                format_json_float(env.sustain_level),
                                format_json_float(env.release_seconds)
                            )
                        } else {
                            String::new()
                        };

                        lines.push(format!(
                            "instrument {} kind=additive harmonics={} envelope={} gain={}",
                            inst.id,
                            harmonics_str,
                            env_str,
                            format_json_float(inst.gain)
                        ));
                    }
                    "sine" | "silence" => {
                        lines.push(format!(
                            "instrument {} kind={} gain={}",
                            inst.id,
                            inst.kind,
                            format_json_float(inst.gain)
                        ));
                    }
                    _ => {
                        lines.push(format!(
                            "instrument {} profile={} gain={}",
                            inst.id,
                            inst.profile_id,
                            format_json_float(inst.gain)
                        ));
                    }
                }
            }
        }

        if !self.tracks.is_empty() {
            lines.push(String::new());
            for track in &self.tracks {
                lines.push(format!("track {} instrument={}", track.id, track.instrument_id));
            }
        }

        if !self.events.is_empty() {
            lines.push(String::new());
            for ev in &self.events {
                match ev.kind {
                    ScoreEventKind::Note => {
                        lines.push(format!(
                            "event {} {} {} note {} velocity={}",
                            ev.track_id,
                            ev.start_tick,
                            ev.duration_tick,
                            ev.notes.join(","),
                            format_json_float(ev.velocity)
                        ));
                    }
                    ScoreEventKind::Hit => {
                        lines.push(format!(
                            "event {} {} {} hit {} velocity={}",
                            ev.track_id,
                            ev.start_tick,
                            ev.duration_tick,
                            ev.hit_id.as_deref().unwrap_or("kick"),
                            format_json_float(ev.velocity)
                        ));
                    }
                    ScoreEventKind::Rest => {
                        lines.push(format!(
                            "event {} {} {} rest",
                            ev.track_id, ev.start_tick, ev.duration_tick
                        ));
                    }
                }
            }
        }

        lines.join("\n")
    }
}
