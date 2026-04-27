const REFERENCE_OCTAVE: i32 = 4;
const REFERENCE_INDEX: i32 = 9;
const REFERENCE_FREQUENCY_HZ: f64 = 440.0;
const SEMITONES_PER_OCTAVE: i32 = 12;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub letter: String,
    pub accidental: String,
    pub octave: i32,
}

impl Note {
    pub fn new(letter: &str, accidental: &str, octave: i32) -> Result<Self, String> {
        let canonical_letter = letter.to_uppercase();
        let spelling = format!("{}{}", canonical_letter, accidental);
        if chromatic_index_for(&spelling).is_none() {
            return Err(format!(
                "Unsupported note spelling {:?}. Only natural notes plus single # or b accidentals are supported.",
                spelling
            ));
        }
        Ok(Self {
            letter: canonical_letter,
            accidental: accidental.to_string(),
            octave,
        })
    }

    pub fn spelling(&self) -> String {
        format!("{}{}", self.letter, self.accidental)
    }

    pub fn chromatic_index(&self) -> i32 {
        chromatic_index_for(&self.spelling()).expect("validated spelling")
    }

    pub fn semitones_from_a4(&self) -> i32 {
        let octave_offset = (self.octave - REFERENCE_OCTAVE) * SEMITONES_PER_OCTAVE;
        let pitch_offset = self.chromatic_index() - REFERENCE_INDEX;
        octave_offset + pitch_offset
    }

    pub fn frequency(&self) -> f64 {
        REFERENCE_FREQUENCY_HZ * 2f64.powf(self.semitones_from_a4() as f64 / SEMITONES_PER_OCTAVE as f64)
    }
}

impl std::fmt::Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.spelling(), self.octave)
    }
}

pub fn parse_note(text: &str) -> Result<Note, String> {
    let mut chars = text.chars();
    let letter = chars.next().ok_or_else(|| invalid_note_message(text))?;
    if !matches!(letter.to_ascii_uppercase(), 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G') {
        return Err(invalid_note_message(text));
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
        return Err(invalid_note_message(text));
    }

    let octave = octave_text
        .parse::<i32>()
        .map_err(|_| invalid_note_message(text))?;
    Note::new(&letter.to_string(), accidental, octave)
}

pub fn note_to_frequency(text: &str) -> Result<f64, String> {
    Ok(parse_note(text)?.frequency())
}

fn invalid_note_message(text: &str) -> String {
    format!(
        "Invalid note {:?}. Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'.",
        text
    )
}
