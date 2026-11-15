# Changelog

All notable changes to the `@coding-adventures/music-analysis` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial safe TypeScript implementation of `MUS08` (Music Understanding, Analysis, and Sample-to-Score Transcription).
- Pure computational capabilities manifest (`required_capabilities.json` = `[]`).
- Equal-tempered pitch inversion (`frequencyToNote`, `frequencyToNoteA440`, `NotePitch`) with cents deviation and zero regex (CWE-1333).
- Spectral & time-domain audio feature extraction:
  - `spectralCentroid` (center of mass / auditory brightness)
  - `spectralFlux` and `spectralFluxSequence` (half-wave rectified and squared spectral variation)
  - `spectralRolloff` (energy percentile cutoff frequency)
  - `zeroCrossingRate` and `zeroCrossingRateFramed`
  - `rmsEnergy` and `rmsEnergyFramed`
- Fundamental pitch estimation:
  - `yinPitch` and `yinPitchTrack` with parabolic interpolation and silence rejection
  - `hpsPitch` (Harmonic Product Spectrum) with logarithmic accumulation and parabolic interpolation
  - `autocorrelationPitch` with normalized cross-correlation and parabolic refinement
- Multi-resolution note onset detection & tempo estimation (`detectOnsets`, `detectOnsetsSamples`, `estimateTempo`):
  - Fuses STFT positive spectral flux with Haar discrete wavelet transform (DWT) detail energy
  - Moving window adaptive thresholding and refractory lockout
  - Inter-onset interval tempo estimation
- Harmonic profile & timbre extraction (`extractHarmonicProfile`):
  - Extracts fundamental amplitude, normalized harmonic partials, and ADSR loudness envelope
  - Yields an `InstrumentProfile` ready for additive re-synthesis in `musical-instruments`
- Sample-to-score transcription (`transcribePcmToPortableScore`, `transcribePcmBuffer`, `transcribeSamples`):
  - Converts raw PCM audio or `pcm-audio::PCMBuffer` into a validated `PortableScore` conforming to `music-machine-score/v2`
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
