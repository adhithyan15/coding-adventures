/**
 * Canonical resource limits, defaults, duration maps, and text score fixtures (MUS01 / MUS03 / MUS04 / MUS07 / VER15).
 * Hardened for CWE-20, CWE-190, CWE-400, CWE-674, CWE-1321, and CWE-1333.
 */

export const MAX_SCORE_BYTES = 1_000_000;
export const MAX_SCORE_LENGTH = 1_000_000;
export const MAX_LINE_LENGTH = 10_000;
export const MAX_LINES = 100_000;
export const MAX_EVENTS = 10_000;
export const MAX_INSTRUMENTS = 128;
export const MAX_TRACKS = 128;
export const MAX_DRUM_HITS = 10_000;
export const MIN_SAMPLE_RATE_HZ = 1000.0;
export const MAX_SAMPLE_RATE_HZ = 192_000.0;
export const MAX_SAMPLE_COUNT = 16_777_216; // 2^24 samples (~32 MiB for 16-bit mono)
export const MAX_DURATION_SECONDS = 600.0; // 10 minutes
export const MAX_PPQ = 100_000;
export const MAX_PARTIALS = 64;
export const MAX_JSON_DEPTH = 32;

export const DEFAULT_TEMPO_BPM = 120.0;
export const DEFAULT_METER = "4/4";
export const DEFAULT_AMPLITUDE = 0.18;
export const DEFAULT_INSTRUMENT_ID = "sine";
export const DEFAULT_PPQ = 480;
export const DEFAULT_SAMPLE_RATE_HZ = 44_100;
export const DEFAULT_MAX_SCORE_LENGTH = 1_000_000;
export const DEFAULT_MAX_LINE_LENGTH = 10_000;
export const DEFAULT_MAX_EVENT_COUNT = 10_000;
export const DEFAULT_MAX_SAMPLE_COUNT = 10_000_000;
export const BEAT_TICK_TOLERANCE = 1e-9;
export const SEMITONES_PER_OCTAVE = 12;

export const PORTABLE_FORMAT_VERSION = "music-machine-score/v2";

export const DURATION_BEATS: Readonly<Record<string, number>> = Object.freeze({
  w: 4.0,
  h: 2.0,
  q: 1.0,
  e: 0.5,
  s: 0.25,
});

export const DIRECTIVE_NAMES: ReadonlySet<string> = Object.freeze(
  new Set([
    "title",
    "tempo",
    "meter",
    "amplitude",
    "sample_rate",
    "instrument",
    "program",
    "drumkit",
  ]),
);

export const BANNED_PROPERTIES: ReadonlySet<string> = Object.freeze(
  new Set(["__proto__", "constructor", "prototype"]),
);

export const CANONICAL_SHARP_SPELLINGS: readonly string[] = Object.freeze([
  "C",
  "C#",
  "D",
  "D#",
  "E",
  "F",
  "F#",
  "G",
  "G#",
  "A",
  "A#",
  "B",
]);

export const HAPPY_BIRTHDAY_TEXT = `title: Happy Birthday
tempo: 120
meter: 3/4
amplitude: 0.18
instrument: sine
sample_rate: 44100

G4/e G4/e | A4/q G4/q C5/q | B4/h R/q |
G4/e G4/e | A4/q G4/q D5/q | C5/h R/q |
G4/e G4/e | G5/q E5/q C5/q | B4/q A4/q R/q |
F5/e F5/e | E5/q C5/q D5/q | C5/h
`;

export const MINI_ORCHESTRA_TEXT = `format: music-machine-score/v2
title: Mini Orchestra
ppq: 120
sample_rate: 16000
tempo 0 96
meter 0 4/4

instrument flute profile=flute_naive gain=0.20
instrument violin profile=violin_naive gain=0.17
instrument piano profile=piano_naive gain=0.14
instrument bass program=33 gain=0.15

track melody instrument=flute
track harmony instrument=violin
track comp instrument=piano
track low instrument=bass

event melody 0 120 note C5 velocity=0.72
event melody 120 120 note D5 velocity=0.72
event melody 240 120 note E5 velocity=0.72
event melody 360 120 note G5 velocity=0.72
event harmony 0 240 note E4 velocity=0.48
event harmony 240 240 note G4 velocity=0.42
event comp 0 240 note C4,E4,G4 velocity=0.42
event comp 240 240 note C4,E4,G4 velocity=0.36
event low 0 480 note C2 velocity=0.56

event melody 480 120 note E5 velocity=0.72
event melody 600 120 note D5 velocity=0.72
event melody 720 120 note C5 velocity=0.72
event melody 840 120 note G4 velocity=0.72
event harmony 480 240 note A4 velocity=0.48
event harmony 720 240 note C5 velocity=0.42
event comp 480 240 note F4,A4,C5 velocity=0.42
event comp 720 240 note F4,A4,C5 velocity=0.36
event low 480 480 note F2 velocity=0.56

event melody 960 120 note A4 velocity=0.72
event melody 1080 120 note C5 velocity=0.72
event melody 1200 120 note D5 velocity=0.72
event melody 1320 120 note E5 velocity=0.72
event harmony 960 240 note B4 velocity=0.48
event harmony 1200 240 note D5 velocity=0.42
event comp 960 240 note G3,B3,D4 velocity=0.42
event comp 1200 240 note G3,B3,D4 velocity=0.36
event low 960 480 note G2 velocity=0.56

event melody 1440 120 note G5 velocity=0.72
event melody 1560 120 note E5 velocity=0.72
event melody 1680 120 note D5 velocity=0.72
event melody 1800 120 note C5 velocity=0.72
event harmony 1440 240 note E4 velocity=0.48
event harmony 1680 240 note B4 velocity=0.42
event comp 1440 240 note C4,E4,G4 velocity=0.42
event comp 1680 240 note C4,E4,G4 velocity=0.36
event low 1440 480 note C2 velocity=0.56
`;

export const PITCHED_PERCUSSION_MIX_TEXT = `format: music-machine-score/v2
title: Pitched Percussion Mix
ppq: 120
sample_rate: 44100
tempo 0 100
meter 0 4/4

instrument flute profile=flute_naive gain=0.12
instrument piano profile=piano_naive gain=0.10
instrument glock profile=glockenspiel_naive gain=0.08
instrument vibes profile=vibraphone_naive gain=0.08
instrument timp profile=timpani_naive gain=0.10

track melody instrument=flute
track comp instrument=piano
track sparkle instrument=glock
track sustain instrument=vibes
track drum instrument=timp

event melody 0 120 note C5 velocity=0.55
event melody 120 120 note D5 velocity=0.55
event melody 240 120 note E5 velocity=0.55
event melody 360 120 note G5 velocity=0.55
event melody 480 120 note E5 velocity=0.55
event melody 600 120 note D5 velocity=0.55
event melody 720 120 note C5 velocity=0.55
event melody 840 240 note G4 velocity=0.55
event melody 1080 120 note A4 velocity=0.55
event melody 1200 120 note C5 velocity=0.55
event melody 1320 120 note E5 velocity=0.55
event melody 1440 120 note E5 velocity=0.55
event melody 1560 120 note G5 velocity=0.55
event melody 1680 120 note E5 velocity=0.55
event melody 1800 120 note D5 velocity=0.55
event melody 1920 240 note C5 velocity=0.55

event comp 0 240 note C4,E4,G4 velocity=0.32
event comp 240 240 note C4,E4,G4 velocity=0.28
event comp 480 240 note F4,A4,C5 velocity=0.32
event comp 720 240 note F4,A4,C5 velocity=0.28
event comp 960 240 note G3,B3,D4 velocity=0.32
event comp 1200 240 note G3,B3,D4 velocity=0.28
event comp 1440 240 note C4,E4,G4 velocity=0.32
event comp 1680 240 note C4,E4,G4 velocity=0.28

event sparkle 0 60 note C6 velocity=0.30
event sparkle 240 60 note G5 velocity=0.24
event sparkle 480 60 note A5 velocity=0.30
event sparkle 720 60 note C6 velocity=0.24
event sparkle 960 60 note D6 velocity=0.30
event sparkle 1200 60 note B5 velocity=0.24
event sparkle 1440 60 note C6 velocity=0.30
event sparkle 1680 60 note G5 velocity=0.24

event sustain 0 480 note G4 velocity=0.18
event sustain 480 480 note A4 velocity=0.18
event sustain 960 480 note B4 velocity=0.18
event sustain 1440 480 note G4 velocity=0.18
`;
