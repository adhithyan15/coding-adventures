//! Canonical resource limits and defaults (CWE-400 / CWE-190 / CWE-20).

/// Maximum allowed score text size in bytes (1 MB).
pub const MAX_SCORE_BYTES: usize = 1_000_000;

/// Maximum allowed score text length in characters (1,000,000).
pub const MAX_SCORE_LENGTH: usize = 1_000_000;

/// Maximum allowed line length in characters (10,000).
pub const MAX_LINE_LENGTH: usize = 10_000;

/// Maximum allowed line count in a score (100,000).
pub const MAX_LINES: usize = 100_000;

/// Maximum allowed event count in a score (10,000).
pub const MAX_EVENTS: usize = 10_000;

/// Maximum allowed instrument count in a score (128).
pub const MAX_INSTRUMENTS: usize = 128;

/// Maximum allowed track count in a score (128).
pub const MAX_TRACKS: usize = 128;

/// Maximum allowed drum hit events in a score (10,000).
pub const MAX_DRUM_HITS: usize = 10_000;

/// Minimum allowable audio sample rate in Hertz (1000.0).
pub const MIN_SAMPLE_RATE_HZ: f64 = 1000.0;

/// Maximum allowable audio sample rate in Hertz (192,000.0).
pub const MAX_SAMPLE_RATE_HZ: f64 = 192_000.0;

/// Maximum allowed sample count in a single rendered score (2^24 = 16,777,216, ~32 MB).
pub const MAX_SAMPLE_COUNT: usize = 16_777_216;

/// Maximum allowed total score duration in seconds (600.0 = 10 minutes).
pub const MAX_DURATION_SECONDS: f64 = 600.0;

/// Maximum allowable PPQ (pulses per quarter note) (100,000).
pub const MAX_PPQ: u32 = 100_000;

/// Maximum allowable harmonic partial count per additive instrument (64).
pub const MAX_PARTIALS: usize = 64;

/// Maximum allowable JSON nesting depth to prevent stack exhaustion (CWE-674).
pub const MAX_JSON_DEPTH: usize = 32;

/// Default score tempo in beats per minute (120.0).
pub const DEFAULT_TEMPO_BPM: f64 = 120.0;

/// Default score meter ("4/4").
pub const DEFAULT_METER: &str = "4/4";

/// Default amplitude (0.18).
pub const DEFAULT_AMPLITUDE: f64 = 0.18;

/// Default instrument profile identifier ("sine").
pub const DEFAULT_INSTRUMENT_ID: &str = "sine";

/// Default PPQ for portable multi-track scores (480).
pub const DEFAULT_PPQ: u32 = 480;

/// Default audio sampling rate in Hertz (44,100).
pub const DEFAULT_SAMPLE_RATE_HZ: u32 = 44_100;

/// Default max score length.
pub const DEFAULT_MAX_SCORE_LENGTH: usize = 1_000_000;

/// Default max line length.
pub const DEFAULT_MAX_LINE_LENGTH: usize = 10_000;

/// Default max event count.
pub const DEFAULT_MAX_EVENT_COUNT: usize = 10_000;

/// Default maximum allowed rendered sample count (10 million).
pub const DEFAULT_MAX_SAMPLE_COUNT: usize = 10_000_000;

/// Canonical format identifier for V2 portable scores.
pub const PORTABLE_FORMAT_VERSION: &str = "music-machine-score/v2";

/// Canonical Happy Birthday score text fixture.
pub const HAPPY_BIRTHDAY_TEXT: &str = "\
title: Happy Birthday\n\
tempo: 120\n\
meter: 3/4\n\
amplitude: 0.18\n\
instrument: sine\n\
sample_rate: 44100\n\
\n\
G4/e G4/e | A4/q G4/q C5/q | B4/h R/q |\n\
G4/e G4/e | A4/q G4/q D5/q | C5/h R/q |\n\
G4/e G4/e | G5/q E5/q C5/q | B4/q A4/q R/q |\n\
F5/e F5/e | E5/q C5/q D5/q | C5/h\n";
