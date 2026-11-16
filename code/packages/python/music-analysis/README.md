# coding-adventures-music-analysis

Pure Python music understanding, feature extraction, pitch tracking, onset detection, harmonic profile estimation, and sample-to-score transcription.

Part of the Coding Adventures music & audio processing stack (`MUS08`).

## Features

- **Tuning and Note Inversion**: `frequency_to_note(hz, a4_hz=440.0)` returning canonical note name, MIDI semitone index, and cents deviation.
- **Spectral and Time-Domain Features**: `spectral_centroid`, `spectral_flux`, `spectral_rolloff`, `zero_crossing_rate`, `rms_energy`.
- **Iterative Pitch Tracking**:
  - YIN algorithm (`yin_pitch`, `yin_pitch_track`) via difference function, cumulative mean normalized difference, and parabolic interpolation.
  - Harmonic Product Spectrum (`hps_pitch`) via spectral downsampling and harmonic reinforcement.
- **Multi-Resolution Onset Detection**: `detect_onsets` fusing spectral flux with wavelet detail energy and adaptive peak picking.
- **Harmonic Profile & Timbre Estimation**: `extract_harmonic_profile` extracting fundamental, overtones, and ADSR envelope as a `MUS05` `InstrumentProfile`.
- **Sample-to-Score Transcription**: `transcribe_pcm_to_portable_score` mapping PCM audio buffers to `MUS04` `PortableScore` representations with full round-trip verifiability.
- **VER15 Security Hardened**: Pre-flight bounds (CWE-400), fail-closed finite float validation (CWE-20), strictly iterative execution (CWE-674), zero regex (CWE-1333), and zero ambient capabilities (`[]`).
