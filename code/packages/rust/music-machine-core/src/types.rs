//! Domain data types for text scores and portable multi-track scores.

use std::collections::HashSet;

use musical_instruments::{
    get_instrument, instrument_for_gm_program, AdsrEnvelope, HarmonicPartial,
    InstrumentNoteRender, InstrumentProfile, VariationProfile,
};
use percussion_kit::PercussionHitEvent;

use crate::error::MusicMachineError;
use crate::limits::*;

/// The musical nature of a score event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreEventKind {
    /// A pitched note or chord.
    Note,
    /// Silence.
    Rest,
    /// An unpitched percussion strike.
    Hit,
}

impl ScoreEventKind {
    /// Return the canonical string identifier for this event kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Rest => "rest",
            Self::Hit => "hit",
        }
    }

    /// Parse an event kind from text.
    pub fn from_str_strict(s: &str) -> Result<Self, MusicMachineError> {
        match s {
            "note" => Ok(Self::Note),
            "rest" => Ok(Self::Rest),
            "hit" => Ok(Self::Hit),
            _ => Err(MusicMachineError::ParseError(format!(
                "event kind must be 'note', 'rest', or 'hit', got '{s}'"
            ))),
        }
    }
}

/// One musical instruction in a beginner score (MUS01/MUS03).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreEvent {
    /// Note, rest, or percussion hit.
    pub kind: ScoreEventKind,
    /// Parsed note spelling (e.g. "C4", "G#5"), if kind is Note.
    pub note: Option<String>,
    /// Percussion hit identifier (e.g. "kick", "snare"), if kind is Hit.
    pub hit_id: Option<String>,
    /// Text duration symbol (e.g. "q", "e.", "h").
    pub duration_symbol: String,
    /// Quarter-note beat count (e.g. 1.0, 0.75, 2.0).
    pub beat_count: f64,
    /// Duration in seconds computed from tempo.
    pub duration_seconds: f64,
    /// Original text token (e.g. "G4/e").
    pub source_token: String,
}

impl ScoreEvent {
    /// Validate event parameters.
    pub fn new(
        kind: ScoreEventKind,
        note: Option<String>,
        hit_id: Option<String>,
        duration_symbol: String,
        beat_count: f64,
        duration_seconds: f64,
        source_token: String,
    ) -> Result<Self, MusicMachineError> {
        if !beat_count.is_finite() || beat_count <= 0.0 {
            return Err(MusicMachineError::InvalidArgument(format!(
                "beat_count must be > 0.0, got {beat_count}"
            )));
        }
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
            return Err(MusicMachineError::InvalidArgument(format!(
                "duration_seconds must be > 0.0, got {duration_seconds}"
            )));
        }
        match kind {
            ScoreEventKind::Note => {
                let n = note.ok_or_else(|| {
                    MusicMachineError::InvalidArgument("note events must include a note".into())
                })?;
                Ok(Self {
                    kind,
                    note: Some(n),
                    hit_id: None,
                    duration_symbol,
                    beat_count,
                    duration_seconds,
                    source_token,
                })
            }
            ScoreEventKind::Rest => Ok(Self {
                kind,
                note: None,
                hit_id: None,
                duration_symbol,
                beat_count,
                duration_seconds,
                source_token,
            }),
            ScoreEventKind::Hit => {
                let h = hit_id.ok_or_else(|| {
                    MusicMachineError::InvalidArgument("hit events must include a hit_id".into())
                })?;
                Ok(Self {
                    kind,
                    note: None,
                    hit_id: Some(h),
                    duration_symbol,
                    beat_count,
                    duration_seconds,
                    source_token,
                })
            }
        }
    }
}

/// A parsed single-line score plus global settings (MUS01/MUS03).
#[derive(Debug, Clone, PartialEq)]
pub struct TextScore {
    /// Score title.
    pub title: String,
    /// Global tempo in beats per minute.
    pub tempo_bpm: f64,
    /// Time signature / meter (e.g. "4/4", "3/4").
    pub meter: String,
    /// Global playback amplitude in [0.0, 1.0].
    pub amplitude: f64,
    /// Audio sample rate in Hertz.
    pub sample_rate_hz: u32,
    /// Selected instrument identifier.
    pub instrument_id: String,
    /// General MIDI program number in [1, 128], if specified.
    pub gm_program: Option<u8>,
    /// Sequential list of score events.
    pub events: Vec<ScoreEvent>,
    /// Drumkit identifier, if drumkit is selected.
    pub drumkit: Option<String>,
}

impl TextScore {
    /// Return seconds per quarter-note beat.
    pub fn seconds_per_beat(&self) -> f64 {
        60.0 / self.tempo_bpm
    }

    /// Return total score duration in quarter-note beats.
    pub fn total_duration_beats(&self) -> f64 {
        self.events.iter().map(|e| e.beat_count).sum()
    }

    /// Return total score duration in seconds.
    pub fn total_duration_seconds(&self) -> f64 {
        self.events.iter().map(|e| e.duration_seconds).sum()
    }
}

/// A tempo change directive in a portable multi-track score.
#[derive(Debug, Clone, PartialEq)]
pub struct TempoEvent {
    /// Start tick position on the PPQ grid.
    pub start_tick: u64,
    /// Tempo in beats per minute.
    pub bpm: f64,
}

/// A time signature change directive in a portable multi-track score.
#[derive(Debug, Clone, PartialEq)]
pub struct MeterEvent {
    /// Start tick position on the PPQ grid.
    pub start_tick: u64,
    /// Meter string (e.g. "4/4").
    pub meter: String,
}

/// An instrument declaration in a portable multi-track score.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentDeclaration {
    /// Unique track instrument identifier (e.g. "lead", "drums").
    pub id: String,
    /// Profile or synthesis kind identifier.
    pub profile_id: String,
    /// Output gain in [0.0, 1.0].
    pub gain: f64,
    /// Kind: "preset", "additive", "drumkit", "sine", or "silence".
    pub kind: String,
    /// Drum kit name if kind == "drumkit" (e.g. "standard_kit").
    pub kit: Option<String>,
    /// Harmonic overtone partials if kind == "additive".
    pub harmonics: Option<Vec<HarmonicPartial>>,
    /// ADSR loudness envelope if kind == "additive".
    pub envelope: Option<AdsrEnvelope>,
    /// Pre-compiled custom instrument profile if kind == "additive".
    pub custom_profile: Option<InstrumentProfile>,
}

impl InstrumentDeclaration {
    /// Construct a new instrument declaration with domain validation.
    pub fn new(
        id: &str,
        profile_id: &str,
        gain: f64,
        kind: &str,
        kit: Option<String>,
        harmonics: Option<Vec<HarmonicPartial>>,
        envelope: Option<AdsrEnvelope>,
    ) -> Result<Self, MusicMachineError> {
        validate_identifier("instrument id", id)?;
        validate_unit_float("gain", gain)?;

        let (actual_kind, actual_profile_id, actual_kit, custom_profile) = if kind == "drumkit"
            || profile_id.starts_with("drumkit")
            || kit.is_some()
        {
            let kit_name = kit.unwrap_or_else(|| "standard_kit".to_string());
            let normalized_kit = if kit_name == "standard" {
                "standard_kit".to_string()
            } else {
                kit_name
            };
            ("drumkit".to_string(), "drumkit".to_string(), Some(normalized_kit), None)
        } else if kind == "additive" || harmonics.is_some() {
            let h = harmonics.clone().ok_or_else(|| {
                MusicMachineError::InvalidArgument("kind=additive must provide harmonics".into())
            })?;
            if h.is_empty() {
                return Err(MusicMachineError::InvalidArgument(
                    "kind=additive must provide at least one harmonic partial".into(),
                ));
            }
            if h.len() > MAX_PARTIALS {
                return Err(MusicMachineError::BudgetExceeded(format!(
                    "harmonics count {} exceeds MAX_PARTIALS={MAX_PARTIALS}",
                    h.len()
                )));
            }
            let env = envelope.unwrap_or_else(|| {
                AdsrEnvelope::new(0.005, 0.010, 0.90, 0.030).expect("valid default additive envelope")
            });
            let profile = InstrumentProfile::new(
                id,
                id,
                "additive",
                1.0,
                h,
                env,
                VariationProfile::default(),
                None,
                "custom",
            )?;
            ("additive".to_string(), id.to_string(), None, Some(profile))
        } else if kind == "sine" || kind == "silence" {
            (kind.to_string(), kind.to_string(), None, None)
        } else {
            let res = resolve_instrument_id(Some(profile_id), None)?;
            ("preset".to_string(), res, None, None)
        };

        Ok(Self {
            id: id.to_string(),
            profile_id: actual_profile_id,
            gain,
            kind: actual_kind,
            kit: actual_kit,
            harmonics,
            envelope,
            custom_profile,
        })
    }
}

/// A track declaration linking a track identifier to an instrument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackDeclaration {
    /// Unique track identifier (e.g. "melody", "bass").
    pub id: String,
    /// Associated instrument identifier.
    pub instrument_id: String,
}

impl TrackDeclaration {
    /// Construct and validate a track declaration.
    pub fn new(id: &str, instrument_id: &str) -> Result<Self, MusicMachineError> {
        validate_identifier("track id", id)?;
        validate_identifier("instrument_id", instrument_id)?;
        Ok(Self {
            id: id.to_string(),
            instrument_id: instrument_id.to_string(),
        })
    }
}

/// An explicit timed note, chord, rest, or percussion hit event.
#[derive(Debug, Clone, PartialEq)]
pub struct PortableScoreEvent {
    /// Track this event belongs to.
    pub track_id: String,
    /// Start tick position.
    pub start_tick: u64,
    /// Duration in ticks.
    pub duration_tick: u64,
    /// Event kind.
    pub kind: ScoreEventKind,
    /// List of note pitch spellings for Note events.
    pub notes: Vec<String>,
    /// Velocity / loudness in [0.0, 1.0].
    pub velocity: f64,
    /// Original occurrence index in score text for stable tie-breaking.
    pub source_order: usize,
    /// Percussion hit identifier for Hit events.
    pub hit_id: Option<String>,
}

impl PortableScoreEvent {
    /// Construct and validate a portable score event.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        track_id: &str,
        start_tick: u64,
        duration_tick: u64,
        kind: ScoreEventKind,
        notes: Vec<String>,
        velocity: f64,
        source_order: usize,
        hit_id: Option<String>,
    ) -> Result<Self, MusicMachineError> {
        validate_identifier("track_id", track_id)?;
        if duration_tick == 0 {
            return Err(MusicMachineError::InvalidArgument(
                "duration_tick must be > 0".into(),
            ));
        }
        start_tick.checked_add(duration_tick).ok_or_else(|| {
            MusicMachineError::BudgetExceeded(format!(
                "event start_tick {start_tick} + duration_tick {duration_tick} overflows u64"
            ))
        })?;
        validate_unit_float("velocity", velocity)?;

        match kind {
            ScoreEventKind::Note => {
                if notes.is_empty() {
                    return Err(MusicMachineError::InvalidArgument(
                        "note events must include at least one note".into(),
                    ));
                }
                // Validate all note spellings through musical_instruments / note_frequency
                for note in &notes {
                    musical_instruments::parse_note(note).map_err(|e| {
                        MusicMachineError::InvalidArgument(format!("invalid note spelling '{note}': {e}"))
                    })?;
                }
                Ok(Self {
                    track_id: track_id.to_string(),
                    start_tick,
                    duration_tick,
                    kind,
                    notes,
                    velocity,
                    source_order,
                    hit_id: None,
                })
            }
            ScoreEventKind::Rest => {
                if !notes.is_empty() {
                    return Err(MusicMachineError::InvalidArgument(
                        "rest events cannot include notes".into(),
                    ));
                }
                Ok(Self {
                    track_id: track_id.to_string(),
                    start_tick,
                    duration_tick,
                    kind,
                    notes: Vec::new(),
                    velocity,
                    source_order,
                    hit_id: None,
                })
            }
            ScoreEventKind::Hit => {
                if !notes.is_empty() {
                    return Err(MusicMachineError::InvalidArgument(
                        "hit events cannot include notes".into(),
                    ));
                }
                let hit = hit_id.ok_or_else(|| {
                    MusicMachineError::InvalidArgument("hit events must include a hit_id".into())
                })?;
                let hit_lower = hit.to_ascii_lowercase();
                if !percussion_kit::ALL_STANDARD_HIT_IDS.contains(&hit_lower.as_str()) {
                    return Err(MusicMachineError::InvalidArgument(format!(
                        "unknown hit_id '{hit_lower}'"
                    )));
                }
                Ok(Self {
                    track_id: track_id.to_string(),
                    start_tick,
                    duration_tick,
                    kind,
                    notes: Vec::new(),
                    velocity,
                    source_order,
                    hit_id: Some(hit_lower),
                })
            }
        }
    }
}

/// A parsed `music-machine-score/v2` multi-track score model.
#[derive(Debug, Clone, PartialEq)]
pub struct PortableScore {
    /// Format version string (must equal `music-machine-score/v2`).
    pub format_version: String,
    /// Score title.
    pub title: String,
    /// Pulses per quarter note (PPQ).
    pub ppq: u32,
    /// Target PCM sample rate in Hertz.
    pub sample_rate_hz: u32,
    /// Chronological list of tempo events.
    pub tempo_events: Vec<TempoEvent>,
    /// Chronological list of meter events.
    pub meter_events: Vec<MeterEvent>,
    /// Declared instruments.
    pub instruments: Vec<InstrumentDeclaration>,
    /// Declared tracks.
    pub tracks: Vec<TrackDeclaration>,
    /// Scheduled events sorted by `(start_tick, source_order)`.
    pub events: Vec<PortableScoreEvent>,
}

impl PortableScore {
    /// Construct and validate a PortableScore with reference checking and sorting.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        format_version: &str,
        title: &str,
        ppq: u32,
        sample_rate_hz: u32,
        tempo_events: Vec<TempoEvent>,
        meter_events: Vec<MeterEvent>,
        instruments: Vec<InstrumentDeclaration>,
        tracks: Vec<TrackDeclaration>,
        events: Vec<PortableScoreEvent>,
    ) -> Result<Self, MusicMachineError> {
        if format_version != PORTABLE_FORMAT_VERSION {
            return Err(MusicMachineError::InvalidArgument(format!(
                "format_version must be '{PORTABLE_FORMAT_VERSION}', got '{format_version}'"
            )));
        }
        if ppq == 0 || ppq > MAX_PPQ {
            return Err(MusicMachineError::InvalidArgument(format!(
                "ppq must be in [1, {MAX_PPQ}], got {ppq}"
            )));
        }
        if (sample_rate_hz as f64) < MIN_SAMPLE_RATE_HZ || (sample_rate_hz as f64) > MAX_SAMPLE_RATE_HZ {
            return Err(MusicMachineError::InvalidArgument(format!(
                "sample_rate_hz must be in [{MIN_SAMPLE_RATE_HZ}, {MAX_SAMPLE_RATE_HZ}], got {sample_rate_hz}"
            )));
        }

        if tempo_events.len() > MAX_EVENTS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "tempo_events count {} exceeds MAX_EVENTS={MAX_EVENTS}",
                tempo_events.len()
            )));
        }
        if meter_events.len() > MAX_EVENTS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "meter_events count {} exceeds MAX_EVENTS={MAX_EVENTS}",
                meter_events.len()
            )));
        }
        if instruments.len() > MAX_INSTRUMENTS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "instruments count {} exceeds MAX_INSTRUMENTS={MAX_INSTRUMENTS}",
                instruments.len()
            )));
        }
        if tracks.len() > MAX_TRACKS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "tracks count {} exceeds MAX_TRACKS={MAX_TRACKS}",
                tracks.len()
            )));
        }
        if events.len() > MAX_EVENTS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "events count {} exceeds MAX_EVENTS={MAX_EVENTS}",
                events.len()
            )));
        }

        // Validate unique instrument IDs
        let mut instrument_ids = HashSet::new();
        for inst in &instruments {
            if !instrument_ids.insert(inst.id.clone()) {
                return Err(MusicMachineError::InvalidArgument(format!(
                    "duplicate instrument id '{}'",
                    inst.id
                )));
            }
        }

        // Validate unique track IDs and track instrument references
        let mut track_ids = HashSet::new();
        for track in &tracks {
            if !track_ids.insert(track.id.clone()) {
                return Err(MusicMachineError::InvalidArgument(format!(
                    "duplicate track id '{}'",
                    track.id
                )));
            }
            if !instrument_ids.contains(&track.instrument_id) {
                return Err(MusicMachineError::InvalidArgument(format!(
                    "track '{}' references unknown instrument '{}'",
                    track.id, track.instrument_id
                )));
            }
        }

        // Validate event track references and tick ranges
        for event in &events {
            if !track_ids.contains(&event.track_id) {
                return Err(MusicMachineError::InvalidArgument(format!(
                    "event references unknown track '{}'",
                    event.track_id
                )));
            }
            event.start_tick.checked_add(event.duration_tick).ok_or_else(|| {
                MusicMachineError::BudgetExceeded(format!(
                    "event '{}' start_tick {} + duration_tick {} overflows u64",
                    event.track_id, event.start_tick, event.duration_tick
                ))
            })?;
        }

        // Ensure sorted tempo events with a tick 0 event
        let mut sorted_tempos = tempo_events;
        sorted_tempos.sort_by_key(|e| e.start_tick);
        if sorted_tempos.is_empty() || sorted_tempos[0].start_tick != 0 {
            sorted_tempos.insert(
                0,
                TempoEvent {
                    start_tick: 0,
                    bpm: DEFAULT_TEMPO_BPM,
                },
            );
        }

        // Ensure sorted meter events with a tick 0 event
        let mut sorted_meters = meter_events;
        sorted_meters.sort_by_key(|e| e.start_tick);
        if sorted_meters.is_empty() || sorted_meters[0].start_tick != 0 {
            sorted_meters.insert(
                0,
                MeterEvent {
                    start_tick: 0,
                    meter: DEFAULT_METER.to_string(),
                },
            );
        }

        // Sort events by (start_tick, source_order)
        let mut sorted_events = events;
        sorted_events.sort_by_key(|e| (e.start_tick, e.source_order));

        Ok(Self {
            format_version: format_version.to_string(),
            title: title.to_string(),
            ppq,
            sample_rate_hz,
            tempo_events: sorted_tempos,
            meter_events: sorted_meters,
            instruments,
            tracks,
            events: sorted_events,
        })
    }
}

/// Rendered PCM audio and notes for a simple TextScore.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedScore {
    /// Source score definition.
    pub score: TextScore,
    /// Joined PCM audio buffer.
    pub pcm_buffer: pcm_audio::PcmAudio,
    /// Inspectable rendered notes.
    pub rendered_notes: Vec<InstrumentNoteRender>,
    /// Inspectable rendered percussion hits.
    pub rendered_hits: Vec<PercussionHitEvent>,
    /// Count of mixed samples saturated to 16-bit range.
    pub clipped_sample_count: usize,
}

/// Rendered PCM audio and notes for a portable multi-track score.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedPortableScore {
    /// Source score definition.
    pub score: PortableScore,
    /// Joined PCM audio buffer.
    pub pcm_buffer: pcm_audio::PcmAudio,
    /// Inspectable rendered notes.
    pub rendered_notes: Vec<InstrumentNoteRender>,
    /// Inspectable rendered percussion hits.
    pub rendered_hits: Vec<PercussionHitEvent>,
    /// Count of mixed samples saturated to 16-bit range.
    pub clipped_sample_count: usize,
}

// ============================================================================
// Helper Validation Functions (VER15 / CWE-1333 / CWE-20)
// ============================================================================

/// Check text matches ^[A-Za-z_][A-Za-z0-9_-]*$ linearly (CWE-1333).
pub fn is_identifier(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let first = bytes[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    for &ch in &bytes[1..] {
        if !(ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-') {
            return false;
        }
    }
    true
}

/// Validate identifier string.
pub fn validate_identifier(name: &str, text: &str) -> Result<(), MusicMachineError> {
    if !is_identifier(text) {
        return Err(MusicMachineError::InvalidArgument(format!(
            "{name} must be an identifier matching [A-Za-z_][A-Za-z0-9_-]*, got '{text}'"
        )));
    }
    Ok(())
}

/// Check meter matches ^([1-9][0-9]*)/([1-9][0-9]*)$ linearly.
pub fn is_valid_meter(text: &str) -> bool {
    let mut parts = text.split('/');
    let num_str = match parts.next() {
        Some(s) => s,
        None => return false,
    };
    let den_str = match parts.next() {
        Some(s) => s,
        None => return false,
    };
    if parts.next().is_some() {
        return false;
    }
    if num_str.is_empty() || den_str.is_empty() {
        return false;
    }
    if num_str.starts_with('0') || !num_str.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if den_str.starts_with('0') || !den_str.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    true
}

/// Validate meter text.
pub fn validate_meter(text: &str) -> Result<(), MusicMachineError> {
    if !is_valid_meter(text) {
        return Err(MusicMachineError::InvalidArgument(format!(
            "meter must look like positive beats/beat-unit, got '{text}'"
        )));
    }
    Ok(())
}

/// Validate finite float.
pub fn validate_finite(name: &str, val: f64) -> Result<f64, MusicMachineError> {
    if !val.is_finite() {
        return Err(MusicMachineError::InvalidArgument(format!(
            "{name} must be a finite number, got {val}"
        )));
    }
    Ok(val)
}

/// Validate unit float in [0.0, 1.0].
pub fn validate_unit_float(name: &str, val: f64) -> Result<f64, MusicMachineError> {
    validate_finite(name, val)?;
    if !(0.0..=1.0).contains(&val) {
        return Err(MusicMachineError::InvalidArgument(format!(
            "{name} must be in [0.0, 1.0], got {val}"
        )));
    }
    Ok(val)
}

/// Return quarter-note beat count for a duration symbol.
pub fn beats_for_duration_symbol(symbol: &str) -> Result<f64, MusicMachineError> {
    if symbol.is_empty() {
        return Err(MusicMachineError::InvalidArgument(
            "duration symbol must be non-empty text".into(),
        ));
    }
    let dots = symbol.chars().filter(|&c| c == '.').count();
    if dots > 1 {
        return Err(MusicMachineError::InvalidArgument(format!(
            "duration symbol '{symbol}' may have at most one dot"
        )));
    }
    let is_dotted = symbol.ends_with('.');
    let base_symbol = if is_dotted {
        &symbol[..symbol.len() - 1]
    } else {
        symbol
    };

    let base_beats = match base_symbol {
        "w" => 4.0,
        "h" => 2.0,
        "q" => 1.0,
        "e" => 0.5,
        "s" => 0.25,
        _ => {
            return Err(MusicMachineError::InvalidArgument(format!(
                "unknown duration symbol '{symbol}'"
            )))
        }
    };

    Ok(if is_dotted {
        base_beats * 1.5
    } else {
        base_beats
    })
}

/// Resolve score instrument settings to a concrete profile ID.
pub fn resolve_instrument_id(
    instrument_id: Option<&str>,
    gm_program: Option<u8>,
) -> Result<String, MusicMachineError> {
    if instrument_id.is_some() && gm_program.is_some() {
        return Err(MusicMachineError::InvalidArgument(
            "score may specify instrument or program, not both".into(),
        ));
    }
    if let Some(prog) = gm_program {
        let p = instrument_for_gm_program(prog)?;
        return Ok(p.id);
    }
    match instrument_id {
        Some("drumkit") | Some("standard_kit") | Some("standard") => Ok("drumkit".to_string()),
        Some(id) => {
            let p = get_instrument(id)?;
            Ok(p.id)
        }
        None => Ok(DEFAULT_INSTRUMENT_ID.to_string()),
    }
}
