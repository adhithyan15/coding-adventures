/**
 * General MIDI 128 melodic program catalog and mapping (MUS05 §4).
 */

import { getInstrument } from "./presets.js";
import type { InstrumentProfile } from "./profile.js";
import { validatePositiveInt } from "./validation.js";

export class GMProgram {
  readonly program: number;
  readonly name: string;
  readonly family: string;
  readonly instrumentId: string;

  constructor(
    program: number,
    name: string,
    family: string,
    instrumentId: string,
  ) {
    const prog = validatePositiveInt("program", program);
    if (prog < 1 || prog > 128) {
      throw new RangeError(`program must be in [1, 128], got ${String(prog)}`);
    }
    if (typeof name !== "string" || name.trim().length === 0) {
      throw new TypeError("name must be a non-empty string");
    }
    if (typeof family !== "string" || family.trim().length === 0) {
      throw new TypeError("family must be a non-empty string");
    }
    if (typeof instrumentId !== "string" || instrumentId.trim().length === 0) {
      throw new TypeError("instrumentId must be a non-empty string");
    }

    this.program = prog;
    this.name = name;
    this.family = family;
    this.instrumentId = instrumentId;
  }
}

const GM_ROWS: readonly [number, string, string, string][] = [
  [1, "Acoustic Grand Piano", "piano", "piano_naive"],
  [2, "Bright Acoustic Piano", "piano", "piano_naive"],
  [3, "Electric Grand Piano", "piano", "piano_naive"],
  [4, "Honky-tonk Piano", "piano", "piano_naive"],
  [5, "Electric Piano 1", "piano", "piano_naive"],
  [6, "Electric Piano 2", "piano", "piano_naive"],
  [7, "Harpsichord", "piano", "pluck_naive"],
  [8, "Clavinet", "piano", "pluck_naive"],
  [9, "Celesta", "chromatic percussion", "celesta_naive"],
  [10, "Glockenspiel", "chromatic percussion", "glockenspiel_naive"],
  [11, "Music Box", "chromatic percussion", "mallet_naive"],
  [12, "Vibraphone", "chromatic percussion", "vibraphone_naive"],
  [13, "Marimba", "chromatic percussion", "marimba_naive"],
  [14, "Xylophone", "chromatic percussion", "xylophone_naive"],
  [15, "Tubular Bells", "chromatic percussion", "tubular_bells_naive"],
  [16, "Dulcimer", "chromatic percussion", "pluck_naive"],
  [17, "Drawbar Organ", "organ", "organ_naive"],
  [18, "Percussive Organ", "organ", "organ_naive"],
  [19, "Rock Organ", "organ", "organ_naive"],
  [20, "Church Organ", "organ", "organ_naive"],
  [21, "Reed Organ", "organ", "organ_naive"],
  [22, "Accordion", "organ", "organ_naive"],
  [23, "Harmonica", "organ", "organ_naive"],
  [24, "Tango Accordion", "organ", "organ_naive"],
  [25, "Acoustic Guitar (nylon)", "guitar", "pluck_naive"],
  [26, "Acoustic Guitar (steel)", "guitar", "pluck_naive"],
  [27, "Electric Guitar (jazz)", "guitar", "pluck_naive"],
  [28, "Electric Guitar (clean)", "guitar", "pluck_naive"],
  [29, "Electric Guitar (muted)", "guitar", "pluck_naive"],
  [30, "Overdriven Guitar", "guitar", "synth_lead_naive"],
  [31, "Distortion Guitar", "guitar", "synth_lead_naive"],
  [32, "Guitar Harmonics", "guitar", "pluck_naive"],
  [33, "Acoustic Bass", "bass", "pluck_naive"],
  [34, "Electric Bass (finger)", "bass", "pluck_naive"],
  [35, "Electric Bass (pick)", "bass", "pluck_naive"],
  [36, "Fretless Bass", "bass", "violin_naive"],
  [37, "Slap Bass 1", "bass", "pluck_naive"],
  [38, "Slap Bass 2", "bass", "pluck_naive"],
  [39, "Synth Bass 1", "bass", "synth_lead_naive"],
  [40, "Synth Bass 2", "bass", "synth_lead_naive"],
  [41, "Violin", "strings", "violin_naive"],
  [42, "Viola", "strings", "violin_naive"],
  [43, "Cello", "strings", "violin_naive"],
  [44, "Contrabass", "strings", "violin_naive"],
  [45, "Tremolo Strings", "strings", "violin_naive"],
  [46, "Pizzicato Strings", "strings", "pluck_naive"],
  [47, "Orchestral Harp", "strings", "pluck_naive"],
  [48, "Timpani", "strings", "timpani_naive"],
  [49, "String Ensemble 1", "ensemble", "violin_naive"],
  [50, "String Ensemble 2", "ensemble", "violin_naive"],
  [51, "SynthStrings 1", "ensemble", "synth_pad_naive"],
  [52, "SynthStrings 2", "ensemble", "synth_pad_naive"],
  [53, "Choir Aahs", "ensemble", "synth_pad_naive"],
  [54, "Voice Oohs", "ensemble", "synth_pad_naive"],
  [55, "Synth Voice", "ensemble", "synth_pad_naive"],
  [56, "Orchestra Hit", "ensemble", "brass_naive"],
  [57, "Trumpet", "brass", "brass_naive"],
  [58, "Trombone", "brass", "brass_naive"],
  [59, "Tuba", "brass", "brass_naive"],
  [60, "Muted Trumpet", "brass", "brass_naive"],
  [61, "French Horn", "brass", "brass_naive"],
  [62, "Brass Section", "brass", "brass_naive"],
  [63, "SynthBrass 1", "brass", "synth_lead_naive"],
  [64, "SynthBrass 2", "brass", "synth_lead_naive"],
  [65, "Soprano Sax", "reed", "clarinet_naive"],
  [66, "Alto Sax", "reed", "clarinet_naive"],
  [67, "Tenor Sax", "reed", "clarinet_naive"],
  [68, "Baritone Sax", "reed", "clarinet_naive"],
  [69, "Oboe", "reed", "clarinet_naive"],
  [70, "English Horn", "reed", "clarinet_naive"],
  [71, "Bassoon", "reed", "clarinet_naive"],
  [72, "Clarinet", "reed", "clarinet_naive"],
  [73, "Piccolo", "pipe", "flute_naive"],
  [74, "Flute", "pipe", "flute_naive"],
  [75, "Recorder", "pipe", "flute_naive"],
  [76, "Pan Flute", "pipe", "flute_naive"],
  [77, "Blown Bottle", "pipe", "flute_naive"],
  [78, "Shakuhachi", "pipe", "flute_naive"],
  [79, "Whistle", "pipe", "sine"],
  [80, "Ocarina", "pipe", "flute_naive"],
  [81, "Lead 1 (square)", "synth lead", "synth_lead_naive"],
  [82, "Lead 2 (sawtooth)", "synth lead", "synth_lead_naive"],
  [83, "Lead 3 (calliope)", "synth lead", "synth_lead_naive"],
  [84, "Lead 4 (chiff)", "synth lead", "synth_lead_naive"],
  [85, "Lead 5 (charang)", "synth lead", "synth_lead_naive"],
  [86, "Lead 6 (voice)", "synth lead", "synth_pad_naive"],
  [87, "Lead 7 (fifths)", "synth lead", "synth_lead_naive"],
  [88, "Lead 8 (bass + lead)", "synth lead", "synth_lead_naive"],
  [89, "Pad 1 (new age)", "synth pad", "synth_pad_naive"],
  [90, "Pad 2 (warm)", "synth pad", "synth_pad_naive"],
  [91, "Pad 3 (polysynth)", "synth pad", "synth_pad_naive"],
  [92, "Pad 4 (choir)", "synth pad", "synth_pad_naive"],
  [93, "Pad 5 (bowed)", "synth pad", "synth_pad_naive"],
  [94, "Pad 6 (metallic)", "synth pad", "synth_pad_naive"],
  [95, "Pad 7 (halo)", "synth pad", "synth_pad_naive"],
  [96, "Pad 8 (sweep)", "synth pad", "synth_pad_naive"],
  [97, "FX 1 (rain)", "synth effects", "effect_naive"],
  [98, "FX 2 (soundtrack)", "synth effects", "effect_naive"],
  [99, "FX 3 (crystal)", "synth effects", "effect_naive"],
  [100, "FX 4 (atmosphere)", "synth effects", "effect_naive"],
  [101, "FX 5 (brightness)", "synth effects", "effect_naive"],
  [102, "FX 6 (goblins)", "synth effects", "effect_naive"],
  [103, "FX 7 (echoes)", "synth effects", "effect_naive"],
  [104, "FX 8 (sci-fi)", "synth effects", "effect_naive"],
  [105, "Sitar", "ethnic", "pluck_naive"],
  [106, "Banjo", "ethnic", "pluck_naive"],
  [107, "Shamisen", "ethnic", "pluck_naive"],
  [108, "Koto", "ethnic", "pluck_naive"],
  [109, "Kalimba", "ethnic", "kalimba_naive"],
  [110, "Bagpipe", "ethnic", "clarinet_naive"],
  [111, "Fiddle", "ethnic", "violin_naive"],
  [112, "Shanai", "ethnic", "clarinet_naive"],
  [113, "Tinkle Bell", "percussive", "glockenspiel_naive"],
  [114, "Agogo", "percussive", "glockenspiel_naive"],
  [115, "Steel Drums", "percussive", "vibraphone_naive"],
  [116, "Woodblock", "percussive", "mallet_naive"],
  [117, "Taiko Drum", "percussive", "mallet_naive"],
  [118, "Melodic Tom", "percussive", "timpani_naive"],
  [119, "Synth Drum", "percussive", "effect_naive"],
  [120, "Reverse Cymbal", "percussive", "effect_naive"],
  [121, "Guitar Fret Noise", "sound effects", "effect_naive"],
  [122, "Breath Noise", "sound effects", "effect_naive"],
  [123, "Seashore", "sound effects", "effect_naive"],
  [124, "Bird Tweet", "sound effects", "effect_naive"],
  [125, "Telephone Ring", "sound effects", "effect_naive"],
  [126, "Helicopter", "sound effects", "effect_naive"],
  [127, "Applause", "sound effects", "effect_naive"],
  [128, "Gunshot", "sound effects", "effect_naive"],
];

export const GM_PROGRAMS: readonly GMProgram[] = Object.freeze(
  GM_ROWS.map(([prog, name, family, instId]) => new GMProgram(prog, name, family, instId)),
);

const GM_MAP: ReadonlyMap<number, GMProgram> = new Map(
  GM_PROGRAMS.map((p) => [p.program, p]),
);

export function getGmProgram(program: number): GMProgram {
  const p = validatePositiveInt("program", program);
  const entry = GM_MAP.get(p);
  if (!entry) {
    throw new RangeError(`Unknown GM program: ${String(program)}. Must be in [1, 128].`);
  }
  return entry;
}

export function instrumentForGmProgram(program: number): InstrumentProfile {
  const gm = getGmProgram(program);
  return getInstrument(gm.instrumentId);
}

export function allGmPrograms(): readonly GMProgram[] {
  return GM_PROGRAMS;
}
