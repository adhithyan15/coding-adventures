# Tier-0 Music Machine Golden Fixture

## Overview

This directory provides canonical Tier-0 test vectors for `music-machine` (`MUS01`, `MUS03`, `MUS04`, `MUS05` §9, `MUS07`).
It enables bit-for-bit and sample-level parity testing across Python, Rust, and TypeScript implementations of the music synthesis engine.

## Fixture Files

- `canonical_score.txt`: Canonical human-readable portable score (`music-machine-score/v2`).
  Exercises:
  - Multi-track polyphony (melody, chords, bass, rhythm).
  - Pitched naive instrument profiles (`flute_naive`, `piano_naive`).
  - Custom additive synthesis (`kind=additive harmonics=1:1.0,2:0.5 envelope=0.01,0.05,0.7,0.1 gain=0.5`).
  - Standard drum kit unpitched percussion (`kind=drumkit kit=standard_kit gain=0.8`).
  - Hi-hat choke attenuation (open hi-hat followed by closed hi-hat with 15 ms release choke).
- `canonical_score.json`: Deterministic canonical JSON serialization of the parsed `PortableScore`.
- `expected_render.json`: Exact expected synthesis PCM metrics, including SHA-256 digest of little-endian signed 16-bit mono PCM bytes, sample counts, and sample values.

## Parity Verification Requirements

Downstream implementations (e.g. Rust crate `coding-adventures-music-machine` and TypeScript package `@coding-adventures/music-machine`) must:
1. Parse `canonical_score.txt` and assert equivalence to `canonical_score.json`.
2. Serialize parsed score to JSON and confirm canonical key order and numeric formatting.
3. Render score to signed 16-bit mono PCM at 44,100 Hz and assert:
   - `total_samples == 96138`
   - `clipped_sample_count == 0`
   - `pcm16_le_sha256 == "15c793ee23103560f177d96fab2e200594041d9c06e7dd4f22b0204254364766"`
   - First 32 and last 32 sample arrays match `expected_render.json`.
