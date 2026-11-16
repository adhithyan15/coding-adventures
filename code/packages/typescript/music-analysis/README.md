# @coding-adventures/music-analysis (MUS08)

Pure, safe TypeScript implementation of **MUS08: Music Understanding, Analysis, and Sample-to-Score Transcription**.

## Overview

Where `MUS00` through `MUS07` define the forward synthesis pipeline (score -> notes -> harmonics -> PCM), `MUS08` provides the inverse layer:
- **Audio Features**: Spectral centroid, spectral flux, spectral rolloff, zero-crossing rate (ZCR), and RMS energy.
- **Pitch Detection**: Iterative YIN time-domain autocorrelation, frequency-domain Harmonic Product Spectrum (HPS), and normalized autocorrelation with parabolic sub-bin/sub-sample interpolation.
- **Onset Detection & Tempo**: Multi-resolution fusion of Short-Time Fourier Transform (STFT) spectral flux and Haar Discrete Wavelet Transform (DWT) detail energy, combined with adaptive standard-deviation thresholding and inter-onset interval tempo estimation.
- **Harmonic Profile Extraction**: Single-note timbre analysis extracting harmonic partials and ADSR envelope, yielding an `InstrumentProfile` ready for additive re-synthesis in `musical-instruments`.
- **Sample-to-Score Transcription**: Converts PCM audio or `PCMBuffer` into a validated `PortableScore` conforming to `music-machine-score/v2`.

## Security & Verification (VER15)

- Pure algorithmic TypeScript with zero Node runtime imports in `src/`.
- `required_capabilities.json`: Declared pure computation (`[]` capabilities).
- **CWE-400 / CWE-190**: Pre-flight bounds (`MAX_SAMPLES`, `MAX_FFT_LEN`, `MAX_FRAMES`, `MAX_PARTIALS`, `MAX_ONSETS`, `MAX_BPM`, `MAX_SAMPLE_RATE_HZ`) validated before memory allocation or loops.
- **CWE-20**: Fail-closed validation for non-finite (`NaN`, `±Inf`) numbers, negative durations/counts, and out-of-range parameters (`validatePositiveInt` / `validateFiniteFloat`). Guard `frameLength <= 1` in `zeroCrossingRateFramed` to prevent `0 / 0 = NaN`.
- **CWE-674**: Strictly iterative algorithms (zero recursion, $O(1)$ stack frames).
- **CWE-1333 / CWE-94**: Zero regular expressions and zero dynamic code execution (`eval`/`Function`).
