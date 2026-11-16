# Changelog

## 0.1.0 - 2026-11-15

### Added
- `frequency_to_note` equal-tempered frequency to note, MIDI index, and cents deviation.
- `spectral_centroid`, `spectral_flux`, `spectral_rolloff`, `zero_crossing_rate`, and `rms_energy` audio feature extractors.
- `yin_pitch` and `yin_pitch_track` strictly iterative time-domain pitch estimation.
- `hps_pitch` strictly iterative Harmonic Product Spectrum frequency-domain pitch estimation.
- `detect_onsets` multi-resolution onset detection fusing STFT spectral flux and DWT wavelet detail energy with adaptive peak picking.
- `extract_harmonic_profile` estimating fundamental frequency, harmonic partials, and ADSR envelope as a `MUS05` `InstrumentProfile`.
- `transcribe_pcm_to_portable_score` sample-to-score transcription returning a `MUS04` `PortableScore`.
- VER15 security hardening against CWE-400, CWE-20, CWE-674, CWE-1333, and pure sandbox capabilities (`[]`).
