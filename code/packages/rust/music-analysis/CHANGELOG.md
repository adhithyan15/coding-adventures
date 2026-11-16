# Changelog

All notable changes to the `coding_adventures_music_analysis` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial safe Rust (`#![forbid(unsafe_code)]`) implementation of `MUS08` (Music Understanding, Analysis, and Sample-to-Score Transcription).
- Pure computational capabilities manifest (`required_capabilities.json` = `[]`).
- Equal-tempered pitch inversion (`frequency_to_note`, `frequency_to_note_a440`, `NotePitch`) with cents deviation and zero regex (CWE-1333).
- Spectral & time-domain audio feature extraction:
  - `spectral_centroid` (center of mass / auditory brightness)
  - `spectral_flux` and `spectral_flux_sequence` (half-wave rectified and squared spectral variation)
  - `spectral_rolloff` (energy percentile cutoff frequency)
  - `zero_crossing_rate` and `zero_crossing_rate_framed`
  - `rms_energy` and `rms_energy_framed`
- Fundamental pitch estimation:
  - `yin_pitch` and `yin_pitch_track` with parabolic interpolation and silence rejection
  - `hps_pitch` (Harmonic Product Spectrum) with logarithmic accumulation and parabolic interpolation
- Multi-resolution note onset detection (`detect_onsets`, `detect_onsets_samples`):
  - Fuses STFT positive spectral flux with Haar discrete wavelet transform (DWT) detail energy
  - Moving window adaptive thresholding and refractory lockout
- Harmonic profile & timbre extraction (`extract_harmonic_profile`):
  - Extracts fundamental amplitude, normalized harmonic partials, and ADSR loudness envelope
  - Yields a `musical_instruments::InstrumentProfile` ready for additive re-synthesis
- Sample-to-score transcription (`transcribe_pcm_to_portable_score`, `transcribe_pcm_buffer`, `transcribe_samples`):
  - Converts raw PCM audio or `pcm_audio::PcmAudio` buffers into a validated `music_machine_core::PortableScore`
  - Rhythmic grid quantization and round-trip synthesis parity
- Robust defensive bounds for CWE-20, CWE-400, CWE-190, CWE-674:
  - `MAX_SAMPLES = 16_777_216`
  - `MAX_FFT_LEN = 1_048_576`
  - `MAX_FRAMES = 1_048_576`
  - `MAX_PARTIALS = 64`
  - `MAX_ONSETS = 65_536`
  - `MAX_BPM = 1000.0`
  - `MAX_SAMPLE_RATE_HZ = 192_000.0`
- Complete test suite: unit tests, round-trip tests, SplitMix64 property-based tests, and hostile input tests.
