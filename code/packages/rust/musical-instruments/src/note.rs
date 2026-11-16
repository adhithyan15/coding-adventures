//! Musical note representation and equal-tempered frequency mapping.

use std::fmt;
use std::str::FromStr;

use crate::error::InstrumentError;

const REFERENCE_OCTAVE: i32 = 4;
const REFERENCE_INDEX: i32 = 9;
const REFERENCE_FREQUENCY_HZ: f64 = 440.0;
const SEMITONES_PER_OCTAVE: i32 = 12;

/// Minimum allowable musical octave (-10) (CWE-190 / CWE-20 guard).
pub const MIN_OCTAVE: i32 = -10;

/// Maximum allowable musical octave (20) (CWE-190 / CWE-20 guard).
pub const MAX_OCTAVE: i32 = 20;

fn chromatic_index_for(spelling: &str) -> Option<i32> {
    match spelling {
        "C" => Some(0),
        "C#" | "Db" => Some(1),
        "D" => Some(2),
        "D#" | "Eb" => Some(3),
        "E" => Some(4),
        "F" => Some(5),
        "F#" | "Gb" => Some(6),
        "G" => Some(7),
        "G#" | "Ab" => Some(8),
        "A" => Some(9),
        "A#" | "Bb" => Some(10),
        "B" => Some(11),
        _ => None,
    }
}

/// A musical note with pitch spelling and octave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub letter: String,
    pub accidental: String,
    pub octave: i32,
}

impl Note {
    /// Construct a new validated note.
    pub fn new(letter: &str, accidental: &str, octave: i32) -> Result<Self, InstrumentError> {
        let canonical_letter = letter.to_uppercase();
        let spelling = format!("{canonical_letter}{accidental}");
        if chromatic_index_for(&spelling).is_none() {
            return Err(InstrumentError::NoteParseError(format!(
                "Unsupported note spelling {spelling:?}. Only natural notes plus single # or b accidentals are supported."
            )));
        }
        if !(MIN_OCTAVE..=MAX_OCTAVE).contains(&octave) {
            return Err(InstrumentError::InvalidParameter {
                name: "octave",
                reason: format!("octave must be in [{MIN_OCTAVE}, {MAX_OCTAVE}], got {octave}"),
            });
        }
        Ok(Self {
            letter: canonical_letter,
            accidental: accidental.to_string(),
            octave,
        })
    }

    /// Return the pitch spelling without octave (e.g. "C", "F#", "Bb").
    pub fn spelling(&self) -> String {
        format!("{}{}", self.letter, self.accidental)
    }

    /// Return the 0..11 chromatic index (C = 0, B = 11).
    pub fn chromatic_index(&self) -> i32 {
        chromatic_index_for(&self.spelling()).expect("validated spelling")
    }

    /// Return semitone offset from A4 (440 Hz) using checked arithmetic (CWE-190).
    pub fn semitones_from_a4(&self) -> i32 {
        let octave_offset = (self.octave as i64 - REFERENCE_OCTAVE as i64) * (SEMITONES_PER_OCTAVE as i64);
        let pitch_offset = (self.chromatic_index() - REFERENCE_INDEX) as i64;
        (octave_offset + pitch_offset) as i32
    }

    /// Return fundamental frequency in Hertz using standard A440 equal temperament.
    pub fn frequency(&self) -> f64 {
        let semitones = self.semitones_from_a4();
        let freq = REFERENCE_FREQUENCY_HZ
            * 2.0_f64.powf(semitones as f64 / SEMITONES_PER_OCTAVE as f64);
        if freq.is_finite() && freq > 0.0 {
            freq
        } else {
            0.0
        }
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.spelling(), self.octave)
    }
}

impl FromStr for Note {
    type Err = InstrumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_note(s)
    }
}

impl TryFrom<&str> for Note {
    type Error = InstrumentError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_note(value)
    }
}

/// Trait for types convertible into a validated [`Note`].
pub trait TryIntoNote {
    /// Convert self into a Note, or return an error.
    fn try_into_note(self) -> Result<Note, InstrumentError>;
}

impl TryIntoNote for Note {
    fn try_into_note(self) -> Result<Note, InstrumentError> {
        Ok(self)
    }
}

impl TryIntoNote for &Note {
    fn try_into_note(self) -> Result<Note, InstrumentError> {
        Ok(self.clone())
    }
}

impl TryIntoNote for &str {
    fn try_into_note(self) -> Result<Note, InstrumentError> {
        parse_note(self)
    }
}

impl TryIntoNote for &String {
    fn try_into_note(self) -> Result<Note, InstrumentError> {
        parse_note(self.as_str())
    }
}

impl TryIntoNote for String {
    fn try_into_note(self) -> Result<Note, InstrumentError> {
        parse_note(self.as_str())
    }
}

/// Parse a note string such as "A4", "C#5", or "Eb3".
pub fn parse_note(text: &str) -> Result<Note, InstrumentError> {
    let mut chars = text.chars();
    let letter = chars
        .next()
        .ok_or_else(|| InstrumentError::NoteParseError(invalid_note_message(text)))?;
    if !matches!(
        letter.to_ascii_uppercase(),
        'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G'
    ) {
        return Err(InstrumentError::NoteParseError(invalid_note_message(text)));
    }

    let rest: String = chars.collect();
    let (accidental, octave_text) = if let Some(stripped) = rest.strip_prefix('#') {
        ("#", stripped)
    } else if let Some(stripped) = rest.strip_prefix('b') {
        ("b", stripped)
    } else {
        ("", rest.as_str())
    };

    if octave_text.is_empty() {
        return Err(InstrumentError::NoteParseError(invalid_note_message(text)));
    }
    if !is_canonical_octave(octave_text) {
        return Err(InstrumentError::NoteParseError(invalid_note_message(text)));
    }

    let octave = octave_text
        .parse::<i32>()
        .map_err(|_| InstrumentError::NoteParseError(invalid_note_message(text)))?;
    Note::new(&letter.to_string(), accidental, octave)
}

fn invalid_note_message(text: &str) -> String {
    format!(
        "Invalid note {text:?}. Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'."
    )
}

fn is_canonical_octave(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}
