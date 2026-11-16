//! General MIDI 128 melodic program catalog and mapping (MUS05 §4).

use crate::error::InstrumentError;
use crate::presets::get_instrument;
use crate::profile::InstrumentProfile;

/// One General-MIDI-style melodic program entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GMProgram {
    /// 1-based program number in [1, 128].
    pub program: u8,
    /// Instrument name in the GM standard.
    pub name: &'static str,
    /// Instrument family classification.
    pub family: &'static str,
    /// Mapped naive instrument preset identifier.
    pub instrument_id: &'static str,
}

impl GMProgram {
    /// Construct a new GM program entry.
    pub fn new(
        program: u8,
        name: &'static str,
        family: &'static str,
        instrument_id: &'static str,
    ) -> Result<Self, InstrumentError> {
        if !(1..=128).contains(&program) {
            return Err(InstrumentError::UnknownProgram(program));
        }
        Ok(Self {
            program,
            name,
            family,
            instrument_id,
        })
    }
}

pub const GM_PROGRAMS: [GMProgram; 128] = [
    GMProgram { program: 1, name: "Acoustic Grand Piano", family: "piano", instrument_id: "piano_naive" },
    GMProgram { program: 2, name: "Bright Acoustic Piano", family: "piano", instrument_id: "piano_naive" },
    GMProgram { program: 3, name: "Electric Grand Piano", family: "piano", instrument_id: "piano_naive" },
    GMProgram { program: 4, name: "Honky-tonk Piano", family: "piano", instrument_id: "piano_naive" },
    GMProgram { program: 5, name: "Electric Piano 1", family: "piano", instrument_id: "piano_naive" },
    GMProgram { program: 6, name: "Electric Piano 2", family: "piano", instrument_id: "piano_naive" },
    GMProgram { program: 7, name: "Harpsichord", family: "piano", instrument_id: "pluck_naive" },
    GMProgram { program: 8, name: "Clavinet", family: "piano", instrument_id: "pluck_naive" },
    GMProgram { program: 9, name: "Celesta", family: "chromatic percussion", instrument_id: "celesta_naive" },
    GMProgram { program: 10, name: "Glockenspiel", family: "chromatic percussion", instrument_id: "glockenspiel_naive" },
    GMProgram { program: 11, name: "Music Box", family: "chromatic percussion", instrument_id: "mallet_naive" },
    GMProgram { program: 12, name: "Vibraphone", family: "chromatic percussion", instrument_id: "vibraphone_naive" },
    GMProgram { program: 13, name: "Marimba", family: "chromatic percussion", instrument_id: "marimba_naive" },
    GMProgram { program: 14, name: "Xylophone", family: "chromatic percussion", instrument_id: "xylophone_naive" },
    GMProgram { program: 15, name: "Tubular Bells", family: "chromatic percussion", instrument_id: "tubular_bells_naive" },
    GMProgram { program: 16, name: "Dulcimer", family: "chromatic percussion", instrument_id: "pluck_naive" },
    GMProgram { program: 17, name: "Drawbar Organ", family: "organ", instrument_id: "organ_naive" },
    GMProgram { program: 18, name: "Percussive Organ", family: "organ", instrument_id: "organ_naive" },
    GMProgram { program: 19, name: "Rock Organ", family: "organ", instrument_id: "organ_naive" },
    GMProgram { program: 20, name: "Church Organ", family: "organ", instrument_id: "organ_naive" },
    GMProgram { program: 21, name: "Reed Organ", family: "organ", instrument_id: "organ_naive" },
    GMProgram { program: 22, name: "Accordion", family: "organ", instrument_id: "organ_naive" },
    GMProgram { program: 23, name: "Harmonica", family: "organ", instrument_id: "organ_naive" },
    GMProgram { program: 24, name: "Tango Accordion", family: "organ", instrument_id: "organ_naive" },
    GMProgram { program: 25, name: "Acoustic Guitar (nylon)", family: "guitar", instrument_id: "pluck_naive" },
    GMProgram { program: 26, name: "Acoustic Guitar (steel)", family: "guitar", instrument_id: "pluck_naive" },
    GMProgram { program: 27, name: "Electric Guitar (jazz)", family: "guitar", instrument_id: "pluck_naive" },
    GMProgram { program: 28, name: "Electric Guitar (clean)", family: "guitar", instrument_id: "pluck_naive" },
    GMProgram { program: 29, name: "Electric Guitar (muted)", family: "guitar", instrument_id: "pluck_naive" },
    GMProgram { program: 30, name: "Overdriven Guitar", family: "guitar", instrument_id: "synth_lead_naive" },
    GMProgram { program: 31, name: "Distortion Guitar", family: "guitar", instrument_id: "synth_lead_naive" },
    GMProgram { program: 32, name: "Guitar Harmonics", family: "guitar", instrument_id: "pluck_naive" },
    GMProgram { program: 33, name: "Acoustic Bass", family: "bass", instrument_id: "pluck_naive" },
    GMProgram { program: 34, name: "Electric Bass (finger)", family: "bass", instrument_id: "pluck_naive" },
    GMProgram { program: 35, name: "Electric Bass (pick)", family: "bass", instrument_id: "pluck_naive" },
    GMProgram { program: 36, name: "Fretless Bass", family: "bass", instrument_id: "violin_naive" },
    GMProgram { program: 37, name: "Slap Bass 1", family: "bass", instrument_id: "pluck_naive" },
    GMProgram { program: 38, name: "Slap Bass 2", family: "bass", instrument_id: "pluck_naive" },
    GMProgram { program: 39, name: "Synth Bass 1", family: "bass", instrument_id: "synth_lead_naive" },
    GMProgram { program: 40, name: "Synth Bass 2", family: "bass", instrument_id: "synth_lead_naive" },
    GMProgram { program: 41, name: "Violin", family: "strings", instrument_id: "violin_naive" },
    GMProgram { program: 42, name: "Viola", family: "strings", instrument_id: "violin_naive" },
    GMProgram { program: 43, name: "Cello", family: "strings", instrument_id: "violin_naive" },
    GMProgram { program: 44, name: "Contrabass", family: "strings", instrument_id: "violin_naive" },
    GMProgram { program: 45, name: "Tremolo Strings", family: "strings", instrument_id: "violin_naive" },
    GMProgram { program: 46, name: "Pizzicato Strings", family: "strings", instrument_id: "pluck_naive" },
    GMProgram { program: 47, name: "Orchestral Harp", family: "strings", instrument_id: "pluck_naive" },
    GMProgram { program: 48, name: "Timpani", family: "strings", instrument_id: "timpani_naive" },
    GMProgram { program: 49, name: "String Ensemble 1", family: "ensemble", instrument_id: "violin_naive" },
    GMProgram { program: 50, name: "String Ensemble 2", family: "ensemble", instrument_id: "violin_naive" },
    GMProgram { program: 51, name: "SynthStrings 1", family: "ensemble", instrument_id: "synth_pad_naive" },
    GMProgram { program: 52, name: "SynthStrings 2", family: "ensemble", instrument_id: "synth_pad_naive" },
    GMProgram { program: 53, name: "Choir Aahs", family: "ensemble", instrument_id: "synth_pad_naive" },
    GMProgram { program: 54, name: "Voice Oohs", family: "ensemble", instrument_id: "synth_pad_naive" },
    GMProgram { program: 55, name: "Synth Voice", family: "ensemble", instrument_id: "synth_pad_naive" },
    GMProgram { program: 56, name: "Orchestra Hit", family: "ensemble", instrument_id: "brass_naive" },
    GMProgram { program: 57, name: "Trumpet", family: "brass", instrument_id: "brass_naive" },
    GMProgram { program: 58, name: "Trombone", family: "brass", instrument_id: "brass_naive" },
    GMProgram { program: 59, name: "Tuba", family: "brass", instrument_id: "brass_naive" },
    GMProgram { program: 60, name: "Muted Trumpet", family: "brass", instrument_id: "brass_naive" },
    GMProgram { program: 61, name: "French Horn", family: "brass", instrument_id: "brass_naive" },
    GMProgram { program: 62, name: "Brass Section", family: "brass", instrument_id: "brass_naive" },
    GMProgram { program: 63, name: "SynthBrass 1", family: "brass", instrument_id: "synth_lead_naive" },
    GMProgram { program: 64, name: "SynthBrass 2", family: "brass", instrument_id: "synth_lead_naive" },
    GMProgram { program: 65, name: "Soprano Sax", family: "reed", instrument_id: "clarinet_naive" },
    GMProgram { program: 66, name: "Alto Sax", family: "reed", instrument_id: "clarinet_naive" },
    GMProgram { program: 67, name: "Tenor Sax", family: "reed", instrument_id: "clarinet_naive" },
    GMProgram { program: 68, name: "Baritone Sax", family: "reed", instrument_id: "clarinet_naive" },
    GMProgram { program: 69, name: "Oboe", family: "reed", instrument_id: "clarinet_naive" },
    GMProgram { program: 70, name: "English Horn", family: "reed", instrument_id: "clarinet_naive" },
    GMProgram { program: 71, name: "Bassoon", family: "reed", instrument_id: "clarinet_naive" },
    GMProgram { program: 72, name: "Clarinet", family: "reed", instrument_id: "clarinet_naive" },
    GMProgram { program: 73, name: "Piccolo", family: "pipe", instrument_id: "flute_naive" },
    GMProgram { program: 74, name: "Flute", family: "pipe", instrument_id: "flute_naive" },
    GMProgram { program: 75, name: "Recorder", family: "pipe", instrument_id: "flute_naive" },
    GMProgram { program: 76, name: "Pan Flute", family: "pipe", instrument_id: "flute_naive" },
    GMProgram { program: 77, name: "Blown Bottle", family: "pipe", instrument_id: "flute_naive" },
    GMProgram { program: 78, name: "Shakuhachi", family: "pipe", instrument_id: "flute_naive" },
    GMProgram { program: 79, name: "Whistle", family: "pipe", instrument_id: "sine" },
    GMProgram { program: 80, name: "Ocarina", family: "pipe", instrument_id: "flute_naive" },
    GMProgram { program: 81, name: "Lead 1 (square)", family: "synth lead", instrument_id: "synth_lead_naive" },
    GMProgram { program: 82, name: "Lead 2 (sawtooth)", family: "synth lead", instrument_id: "synth_lead_naive" },
    GMProgram { program: 83, name: "Lead 3 (calliope)", family: "synth lead", instrument_id: "synth_lead_naive" },
    GMProgram { program: 84, name: "Lead 4 (chiff)", family: "synth lead", instrument_id: "synth_lead_naive" },
    GMProgram { program: 85, name: "Lead 5 (charang)", family: "synth lead", instrument_id: "synth_lead_naive" },
    GMProgram { program: 86, name: "Lead 6 (voice)", family: "synth lead", instrument_id: "synth_pad_naive" },
    GMProgram { program: 87, name: "Lead 7 (fifths)", family: "synth lead", instrument_id: "synth_lead_naive" },
    GMProgram { program: 88, name: "Lead 8 (bass + lead)", family: "synth lead", instrument_id: "synth_lead_naive" },
    GMProgram { program: 89, name: "Pad 1 (new age)", family: "synth pad", instrument_id: "synth_pad_naive" },
    GMProgram { program: 90, name: "Pad 2 (warm)", family: "synth pad", instrument_id: "synth_pad_naive" },
    GMProgram { program: 91, name: "Pad 3 (polysynth)", family: "synth pad", instrument_id: "synth_pad_naive" },
    GMProgram { program: 92, name: "Pad 4 (choir)", family: "synth pad", instrument_id: "synth_pad_naive" },
    GMProgram { program: 93, name: "Pad 5 (bowed)", family: "synth pad", instrument_id: "synth_pad_naive" },
    GMProgram { program: 94, name: "Pad 6 (metallic)", family: "synth pad", instrument_id: "synth_pad_naive" },
    GMProgram { program: 95, name: "Pad 7 (halo)", family: "synth pad", instrument_id: "synth_pad_naive" },
    GMProgram { program: 96, name: "Pad 8 (sweep)", family: "synth pad", instrument_id: "synth_pad_naive" },
    GMProgram { program: 97, name: "FX 1 (rain)", family: "synth effects", instrument_id: "effect_naive" },
    GMProgram { program: 98, name: "FX 2 (soundtrack)", family: "synth effects", instrument_id: "effect_naive" },
    GMProgram { program: 99, name: "FX 3 (crystal)", family: "synth effects", instrument_id: "effect_naive" },
    GMProgram { program: 100, name: "FX 4 (atmosphere)", family: "synth effects", instrument_id: "effect_naive" },
    GMProgram { program: 101, name: "FX 5 (brightness)", family: "synth effects", instrument_id: "effect_naive" },
    GMProgram { program: 102, name: "FX 6 (goblins)", family: "synth effects", instrument_id: "effect_naive" },
    GMProgram { program: 103, name: "FX 7 (echoes)", family: "synth effects", instrument_id: "effect_naive" },
    GMProgram { program: 104, name: "FX 8 (sci-fi)", family: "synth effects", instrument_id: "effect_naive" },
    GMProgram { program: 105, name: "Sitar", family: "ethnic", instrument_id: "pluck_naive" },
    GMProgram { program: 106, name: "Banjo", family: "ethnic", instrument_id: "pluck_naive" },
    GMProgram { program: 107, name: "Shamisen", family: "ethnic", instrument_id: "pluck_naive" },
    GMProgram { program: 108, name: "Koto", family: "ethnic", instrument_id: "pluck_naive" },
    GMProgram { program: 109, name: "Kalimba", family: "ethnic", instrument_id: "kalimba_naive" },
    GMProgram { program: 110, name: "Bagpipe", family: "ethnic", instrument_id: "clarinet_naive" },
    GMProgram { program: 111, name: "Fiddle", family: "ethnic", instrument_id: "violin_naive" },
    GMProgram { program: 112, name: "Shanai", family: "ethnic", instrument_id: "clarinet_naive" },
    GMProgram { program: 113, name: "Tinkle Bell", family: "percussive", instrument_id: "glockenspiel_naive" },
    GMProgram { program: 114, name: "Agogo", family: "percussive", instrument_id: "glockenspiel_naive" },
    GMProgram { program: 115, name: "Steel Drums", family: "percussive", instrument_id: "vibraphone_naive" },
    GMProgram { program: 116, name: "Woodblock", family: "percussive", instrument_id: "mallet_naive" },
    GMProgram { program: 117, name: "Taiko Drum", family: "percussive", instrument_id: "mallet_naive" },
    GMProgram { program: 118, name: "Melodic Tom", family: "percussive", instrument_id: "timpani_naive" },
    GMProgram { program: 119, name: "Synth Drum", family: "percussive", instrument_id: "effect_naive" },
    GMProgram { program: 120, name: "Reverse Cymbal", family: "percussive", instrument_id: "effect_naive" },
    GMProgram { program: 121, name: "Guitar Fret Noise", family: "sound effects", instrument_id: "effect_naive" },
    GMProgram { program: 122, name: "Breath Noise", family: "sound effects", instrument_id: "effect_naive" },
    GMProgram { program: 123, name: "Seashore", family: "sound effects", instrument_id: "effect_naive" },
    GMProgram { program: 124, name: "Bird Tweet", family: "sound effects", instrument_id: "effect_naive" },
    GMProgram { program: 125, name: "Telephone Ring", family: "sound effects", instrument_id: "effect_naive" },
    GMProgram { program: 126, name: "Helicopter", family: "sound effects", instrument_id: "effect_naive" },
    GMProgram { program: 127, name: "Applause", family: "sound effects", instrument_id: "effect_naive" },
    GMProgram { program: 128, name: "Gunshot", family: "sound effects", instrument_id: "effect_naive" },
];

/// Return the complete 128-entry General MIDI melodic program catalog.
pub fn all_gm_programs() -> &'static [GMProgram] {
    &GM_PROGRAMS
}

/// Retrieve a GM melodic program entry by 1-based program number [1, 128].
pub fn get_gm_program(program: u8) -> Result<&'static GMProgram, InstrumentError> {
    if !(1..=128).contains(&program) {
        return Err(InstrumentError::UnknownProgram(program));
    }
    Ok(&GM_PROGRAMS[(program - 1) as usize])
}

/// Return the naive instrument profile mapped to a GM program number.
pub fn instrument_for_gm_program(program: u8) -> Result<InstrumentProfile, InstrumentError> {
    let entry = get_gm_program(program)?;
    get_instrument(entry.instrument_id)
}
