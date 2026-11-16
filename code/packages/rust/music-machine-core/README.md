# music-machine-core

`music-machine-core` implements the core synthesis and sheet music parsing engine for the coding-adventures music stack (`MUS01`, `MUS03`, `MUS04`, `MUS07`, and `VER04`).

## Features

- **Beginner Score Format (MUS01/MUS03)**: Linear-time parsing of melody lines with tempo, meter, amplitude, pitch tokens (`G4/q`), rest tokens (`R/e`), and barlines (`|`).
- **Portable Multi-Track Format (MUS04)**: Full parser for `music-machine-score/v2` multi-track files with multiple instruments, chords, overlapping note events, and drum tracks.
- **Deterministic JSON Serialization**: Bit-for-bit lossless round-trip serialization and deserialization conforming to `canonical_score.json` without any third-party dependencies.
- **Timbre & Percussion Integration**: Native additive harmonics, ADSR envelope shaping, naive presets (`flute_naive`, `piano_naive`, `synth_lead_naive`, etc.) via `musical-instruments`, and standard drum kit synthesis with 15 ms hi-hat choke dynamics via `percussion-kit`.
- **Saturating Polyphonic Mixer**: Overflow-safe PCM mixing with saturation clamping to $[-32768, 32767]$ and exact clipped-sample accounting.
- **WAV File Integration**: Direct export and import of standard RIFF/WAVE files via `wav-sink`.

## Security & Verification Guarantees

- **Safety**: `#![forbid(unsafe_code)]`, zero external third-party dependencies (strictly standard library and workspace sibling crates).
- **CWE-400 (Resource Exhaustion)**: Strict pre-flight budget verification enforcing bounds on score bytes, lines, events, duration, sample rate, partials, and JSON nesting depth before allocation.
- **CWE-190 (Integer Overflow)**: Checked arithmetic on sample counts, timeline indices, ticks, and byte lengths.
- **CWE-20 (Improper Input Validation)**: Finite float checks rejecting `NaN` and `±Inf`.
- **CWE-674 (Uncontrolled Recursion)**: Strictly iterative tokenizers and parsers with explicit depth guards on JSON parsing (`MAX_JSON_DEPTH = 32`).
- **CWE-1333 (ReDoS)**: Zero regular expressions across all parsing paths.

## Usage

```rust
use music_machine_core::{parse_portable_score, render_portable_score_to_pcm, DEFAULT_MAX_SAMPLE_COUNT};

let score_text = r#"
format: music-machine-score/v2
title: Tiny Duet
ppq: 480
tempo 0 120
meter 0 4/4

instrument lead kind=sine gain=0.08
instrument bass kind=sine gain=0.05

track melody instrument=lead
event melody 0 480 note C5 velocity=0.8
event melody 480 480 note D5 velocity=0.8

track bassline instrument=bass
event bassline 0 960 note C3 velocity=0.7
"#;

let score = parse_portable_score(score_text).expect("score parses");
let rendered = render_portable_score_to_pcm(&score, DEFAULT_MAX_SAMPLE_COUNT).expect("renders to PCM");
assert_eq!(rendered.clipped_sample_count, 0);
```
