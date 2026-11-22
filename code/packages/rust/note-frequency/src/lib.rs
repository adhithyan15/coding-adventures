//! # Note Frequency Mapping (MUS00 / VER15)
//!
//! Turns symbolic note labels such as `A4`, `C#5`, and `Db3` into structured
//! [`Note`] values and equal-tempered frequencies in Hertz anchored at `A4 = 440 Hz`:
//!
//! $$f(n) = 440 \cdot 2^{n / 12}$$
//!
//! ## VER15 Security & Safety Guarantees
//!
//! - **Memory Safety**: `#![forbid(unsafe_code)]`, pure safe Rust, zero FFI.
//! - **CWE-400 (Resource Exhaustion)**: [`parse_note`] enforces
//!   [`MAX_NOTE_STRING_LENGTH`] (`32` bytes) **before** any heap allocation,
//!   scanning ASCII bytes in linear time without collecting intermediate strings.
//! - **CWE-190 (Integer Overflow)**: Octave values are bounded to
//!   `[MIN_OCTAVE, MAX_OCTAVE]` (`[-100, 100]`), and [`Note::semitones_from_a4`]
//!   computes offsets in `i64` so even manually constructed struct literals
//!   cannot overflow `i32`.
//! - **CWE-20 (Improper Input Validation)**: Rejects non-finite (`NaN`, `±Inf`)
//!   and out-of-range frequency inputs via [`validate_frequency`] and
//!   [`validate_reference_frequency`].

#![forbid(unsafe_code)]

/// Maximum accepted byte length for a note label (CWE-400 guard).
pub const MAX_NOTE_STRING_LENGTH: usize = 32;

/// Minimum supported octave (inclusive, CWE-190 / exponentiation guard).
pub const MIN_OCTAVE: i32 = -100;

/// Maximum supported octave (inclusive, CWE-190 / exponentiation guard).
pub const MAX_OCTAVE: i32 = 100;

/// Minimum valid physical frequency in Hertz (below `C-100` ~ `1.24e-29` Hz).
pub const MIN_FREQUENCY_HZ: f64 = 1.0e-30;

/// Maximum valid physical frequency in Hertz (above `B100` ~ `3.75e31` Hz).
pub const MAX_FREQUENCY_HZ: f64 = 1.0e32;

/// Minimum valid A4 reference tuning frequency in Hertz.
pub const MIN_REFERENCE_FREQUENCY_HZ: f64 = 1.0;

/// Maximum valid A4 reference tuning frequency in Hertz.
pub const MAX_REFERENCE_FREQUENCY_HZ: f64 = 100_000.0;

const REFERENCE_OCTAVE: i32 = 4;
const REFERENCE_INDEX: i32 = 9;
const REFERENCE_FREQUENCY_HZ: f64 = 440.0;
const SEMITONES_PER_OCTAVE: i32 = 12;

fn chromatic_index_parts(letter: &str, accidental: &str) -> Option<i32> {
    match (letter, accidental) {
        ("C", "") => Some(0),
        ("C", "#") | ("D", "b") => Some(1),
        ("D", "") => Some(2),
        ("D", "#") | ("E", "b") => Some(3),
        ("E", "") => Some(4),
        ("F", "") => Some(5),
        ("F", "#") | ("G", "b") => Some(6),
        ("G", "") => Some(7),
        ("G", "#") | ("A", "b") => Some(8),
        ("A", "") => Some(9),
        ("A", "#") | ("B", "b") => Some(10),
        ("B", "") => Some(11),
        _ => None,
    }
}

/// Validate that a frequency in Hertz is finite and within physical bounds (CWE-20).
pub fn validate_frequency(freq_hz: f64) -> Result<f64, String> {
    if !freq_hz.is_finite() {
        return Err(format!("Frequency must be finite (not NaN or Inf), got {freq_hz}."));
    }
    if !(MIN_FREQUENCY_HZ..=MAX_FREQUENCY_HZ).contains(&freq_hz) {
        return Err(format!(
            "Frequency {freq_hz} Hz out of valid range [{MIN_FREQUENCY_HZ}, {MAX_FREQUENCY_HZ}]."
        ));
    }
    Ok(freq_hz)
}

/// Validate that an A4 reference tuning frequency is finite and within bounds (CWE-20).
pub fn validate_reference_frequency(a4_hz: f64) -> Result<f64, String> {
    if !a4_hz.is_finite() {
        return Err(format!(
            "Reference frequency a4_hz must be finite (not NaN or Inf), got {a4_hz}."
        ));
    }
    if !(MIN_REFERENCE_FREQUENCY_HZ..=MAX_REFERENCE_FREQUENCY_HZ).contains(&a4_hz) {
        return Err(format!(
            "Reference frequency a4_hz ({a4_hz} Hz) out of valid range [{MIN_REFERENCE_FREQUENCY_HZ}, {MAX_REFERENCE_FREQUENCY_HZ}]."
        ));
    }
    Ok(a4_hz)
}

/// A parsed musical note label with canonical uppercase letter, accidental, and octave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Canonical uppercase note letter (`"A"` through `"G"`).
    pub letter: String,
    /// Optional accidental (`""`, `"#"`, or `"b"`).
    pub accidental: String,
    /// Octave number in `[MIN_OCTAVE, MAX_OCTAVE]`.
    pub octave: i32,
}

impl Note {
    /// Construct and validate a [`Note`] from letter, accidental, and octave.
    pub fn new(letter: &str, accidental: &str, octave: i32) -> Result<Self, String> {
        if !(MIN_OCTAVE..=MAX_OCTAVE).contains(&octave) {
            return Err(format!(
                "Octave {octave} out of bounds [{MIN_OCTAVE}, {MAX_OCTAVE}]."
            ));
        }
        if letter.len() != 1 || !matches!(letter.as_bytes()[0].to_ascii_uppercase(), b'A'..=b'G') {
            if letter.len() > MAX_NOTE_STRING_LENGTH {
                return Err(format!(
                    "Unsupported note spelling: letter length {} exceeds maximum {MAX_NOTE_STRING_LENGTH}.",
                    letter.len()
                ));
            }
            return Err(format!(
                "Unsupported note spelling {letter:?}. Letter must be a single ASCII note letter in A..G."
            ));
        }
        if !matches!(accidental, "" | "#" | "b") {
            if accidental.len() > MAX_NOTE_STRING_LENGTH {
                return Err(format!(
                    "Unsupported note spelling: accidental length {} exceeds maximum {MAX_NOTE_STRING_LENGTH}.",
                    accidental.len()
                ));
            }
            let spelling = format!("{}{}", letter.to_ascii_uppercase(), accidental);
            return Err(format!(
                "Unsupported note spelling {spelling:?}. Only natural notes plus single # or b accidentals are supported."
            ));
        }

        let canonical_letter = letter.to_ascii_uppercase();
        if chromatic_index_parts(&canonical_letter, accidental).is_none() {
            let spelling = format!("{canonical_letter}{accidental}");
            return Err(format!(
                "Unsupported note spelling {spelling:?}. Only natural notes plus single # or b accidentals are supported."
            ));
        }
        Ok(Self {
            letter: canonical_letter,
            accidental: accidental.to_string(),
            octave,
        })
    }

    /// Return the pitch spelling without the octave (e.g. `"C#"`, `"Db"`, `"A"`).
    pub fn spelling(&self) -> String {
        format!("{}{}", self.letter, self.accidental)
    }

    /// Return this note's chromatic index if the spelling is valid, or `None`.
    pub fn checked_chromatic_index(&self) -> Option<i32> {
        chromatic_index_parts(&self.letter, &self.accidental)
    }

    /// Return this note's index within the 12-note chromatic octave.
    ///
    /// Returns `0` if a struct literal was manually constructed with an invalid spelling.
    pub fn chromatic_index(&self) -> i32 {
        self.checked_chromatic_index().unwrap_or(0)
    }

    /// Compute the signed semitone offset from `A4` with strict bounds and overflow checks.
    pub fn checked_semitones_from_a4(&self) -> Result<i32, String> {
        if !(MIN_OCTAVE..=MAX_OCTAVE).contains(&self.octave) {
            return Err(format!(
                "Octave {} out of bounds [{MIN_OCTAVE}, {MAX_OCTAVE}].",
                self.octave
            ));
        }
        let idx = self.checked_chromatic_index().ok_or_else(|| {
            if self.letter.len().saturating_add(self.accidental.len()) > MAX_NOTE_STRING_LENGTH {
                "Unsupported note spelling: length exceeds maximum.".to_string()
            } else {
                format!("Unsupported note spelling {:?}.", self.spelling())
            }
        })?;
        let octave_offset = (i64::from(self.octave) - i64::from(REFERENCE_OCTAVE))
            .checked_mul(i64::from(SEMITONES_PER_OCTAVE))
            .ok_or_else(|| "Semitone offset overflow".to_string())?;
        let pitch_offset = i64::from(idx) - i64::from(REFERENCE_INDEX);
        let total = octave_offset
            .checked_add(pitch_offset)
            .ok_or_else(|| "Semitone offset overflow".to_string())?;
        i32::try_from(total).map_err(|_| "Semitone offset out of i32 range".to_string())
    }

    /// Return the signed semitone offset from `A4`.
    ///
    /// Uses `i64` intermediate arithmetic and clamps to `[i32::MIN, i32::MAX]` so
    /// even manually constructed struct literals cannot overflow or panic (CWE-190).
    pub fn semitones_from_a4(&self) -> i32 {
        let idx = self.checked_chromatic_index().unwrap_or(REFERENCE_INDEX);
        let octave_offset = (i64::from(self.octave) - i64::from(REFERENCE_OCTAVE))
            * i64::from(SEMITONES_PER_OCTAVE);
        let pitch_offset = i64::from(idx) - i64::from(REFERENCE_INDEX);
        let total = octave_offset + pitch_offset;
        total.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }

    /// Return the equal-tempered frequency in Hertz anchored at `A4 = 440 Hz`.
    pub fn frequency(&self) -> f64 {
        REFERENCE_FREQUENCY_HZ
            * 2f64.powf(f64::from(self.semitones_from_a4()) / f64::from(SEMITONES_PER_OCTAVE))
    }

    /// Return the validated equal-tempered frequency in Hertz anchored at `A4 = 440 Hz`.
    pub fn checked_frequency(&self) -> Result<f64, String> {
        self.frequency_with_reference(REFERENCE_FREQUENCY_HZ)
    }

    /// Return the equal-tempered frequency in Hertz relative to a validated `a4_hz`.
    pub fn frequency_with_reference(&self, a4_hz: f64) -> Result<f64, String> {
        let ref_hz = validate_reference_frequency(a4_hz)?;
        let semitones = self.checked_semitones_from_a4()?;
        let freq = ref_hz * 2f64.powf(f64::from(semitones) / f64::from(SEMITONES_PER_OCTAVE));
        validate_frequency(freq)
    }
}

impl std::fmt::Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.spelling(), self.octave)
    }
}

/// Parse a note label like `"A4"` or `"C#5"` into a [`Note`].
///
/// Enforces [`MAX_NOTE_STRING_LENGTH`] before allocating and scans ASCII bytes
/// in a single linear pass (CWE-400 / CWE-1333).
pub fn parse_note(text: &str) -> Result<Note, String> {
    if text.is_empty() {
        return Err(invalid_note_message(text));
    }
    if text.len() > MAX_NOTE_STRING_LENGTH {
        return Err(format!(
            "Invalid note: input length {} exceeds maximum {MAX_NOTE_STRING_LENGTH}.",
            text.len()
        ));
    }

    let bytes = text.as_bytes();
    if !matches!(bytes[0].to_ascii_uppercase(), b'A'..=b'G') {
        return Err(invalid_note_message(text));
    }

    let (accidental, octave_text) = match bytes.get(1) {
        Some(b'#') => ("#", &text[2..]),
        Some(b'b') => ("b", &text[2..]),
        _ => ("", &text[1..]),
    };

    if octave_text.is_empty() || !is_canonical_octave(octave_text) {
        return Err(invalid_note_message(text));
    }

    let octave = octave_text
        .parse::<i32>()
        .map_err(|_| invalid_note_message(text))?;
    Note::new(&text[0..1], accidental, octave)
}

/// Parse a note label and return its equal-tempered frequency in Hertz.
pub fn note_to_frequency(text: &str) -> Result<f64, String> {
    parse_note(text)?.checked_frequency()
}

/// Parse a note label and return its frequency relative to a validated `a4_hz`.
pub fn note_to_frequency_with_reference(text: &str, a4_hz: f64) -> Result<f64, String> {
    parse_note(text)?.frequency_with_reference(a4_hz)
}

fn invalid_note_message(text: &str) -> String {
    format!(
        "Invalid note {text:?}. Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'."
    )
}

fn is_canonical_octave(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}
