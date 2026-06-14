# DSP05 — Short-Time Fourier Transform (STFT) / Spectrogram

**Status**: V1 spec (this document is Phase 0).

**Scope**: a new `dsp-stft` crate providing the Short-Time
Fourier Transform and its inverse (sliding-window FFT for
time-frequency analysis), plus the canonical audio-analysis
APIs built on top — magnitude spectrogram, log-spectrogram,
mel-spectrogram, MDCT.  Built on top of `dsp-fft` (FFT
primitives) and `dsp-filters::design` (window functions).

## Why STFT?

The DFT collapses a whole signal into one spectrum, losing
all time localisation.  Real-world audio / speech / music
isn't stationary — the spectrum changes over time, and you
need to know *when* the frequency content shifted.  The STFT
gives you a 2-D time-frequency view:

- **Audio analysis**: spectrograms for speech recognition,
  music transcription, voice activity detection.
- **MFCCs**: the dominant speech feature for decades — log
  of a mel-filterbanked STFT power spectrogram, then DCT.
  This crate plus `dsp-dct` covers the full pipeline.
- **Source separation**: STFT → magnitude/phase manipulation
  → ISTFT round-trips through the time-frequency plane.
- **Audio coding (MP3, AAC, Vorbis)**: built on MDCT (modified
  DCT with overlap-add), which lives in DSP05.
- **Music information retrieval**: chromagrams, tempograms,
  beat tracking — all built on STFT.

After DSP01 (FFT), DSP02 (DCT), DSP03 (FIR/IIR), and DSP04
(convolution), STFT is the natural next layer.  It builds on
all four:

- FFT for the per-frame transform.
- DCT-II for the final cepstral step in MFCC.
- Windowed-sinc design helpers reused for the analysis window.
- (Optional) overlap-add framework can lean on conv1d for the
  reconstruction step.

## STFT — definition

Given a real signal `x[n]` of length `N`, an analysis window
`w[n]` of length `n_fft`, and a hop length `hop_length`:

```text
    STFT[k, m] = Σ_{n=0..n_fft-1}  x[n + m · hop_length] · w[n]
                                    · exp(-2πi · k · n / n_fft)
```

Where `k ∈ [0, n_fft/2 + 1)` (one-sided spectrum for real
input) and `m ∈ [0, num_frames)`.

`num_frames` depends on the boundary mode:

- **Centred** (default, matches librosa / scipy):
  `num_frames = 1 + (N + n_fft - hop_length) / hop_length`
  with the signal zero-padded by `n_fft / 2` on each side
  so frame 0 is centred at sample 0.
- **Strict**: `num_frames = 1 + (N - n_fft) / hop_length`
  with no padding; only frames that fit entirely inside the
  signal are produced.  Simpler but loses end samples.

V1 ships **Strict** mode only (simplest); centred padding is
a Phase 4 / 5 follow-up.

## ISTFT — definition

Inverse STFT reconstructs the time-domain signal via
**overlap-add**:

```text
    x_hat[n] = ( Σ_m  IFFT(STFT[:, m])[n - m · hop_length] · w[n - m · hop_length] )
              / ( Σ_m  w[n - m · hop_length]² )
```

The synthesis window is typically the same as the analysis
window (with the `w²` denominator ensuring **constant overlap
add** — COLA).  Hann and Hamming windows at hop = n_fft / 2
satisfy COLA exactly.

V1's `istft` requires:

- Same window function used on the forward path.
- `hop_length ≤ n_fft` (otherwise gaps between frames).
- An explicit `output_length` parameter (matches scipy /
  librosa).

## V1 scope

**Phase 1**: `dsp-stft` crate skeleton.

**Phase 2**: scalar STFT (forward).

```rust
pub fn stft(signal: &[f32], n_fft: u32, hop_length: u32,
            window: WindowType)
    -> Result<Vec<f32>, StftError>;
```

Output layout: row-major `[num_frames, n_fft/2 + 1, 2]`
(each frame is a length-`(n_fft/2 + 1)` complex spectrum in
interleaved `[re, im]` format).  Flattened to a `Vec<f32>`
of length `num_frames * (n_fft/2 + 1) * 2`.

**Phase 3**: scalar ISTFT (overlap-add reconstruction).

```rust
pub fn istft(spectrogram: &[f32], n_fft: u32, hop_length: u32,
             window: WindowType, output_length: u32)
    -> Result<Vec<f32>, StftError>;
```

`istft(stft(x)) ≈ x` within `1e-4` relative tolerance for
COLA-satisfying window/hop combinations.

**Phase 4**: magnitude / power / log spectrograms.

```rust
pub fn spectrogram(signal: &[f32], n_fft: u32, hop_length: u32,
                   window: WindowType)
    -> Result<Vec<f32>, StftError>;  // |STFT|²

pub fn log_spectrogram(signal, n_fft, hop_length, window)
    -> Result<Vec<f32>, StftError>;   // log(|STFT|² + ε)
```

Each frame is now a length-`(n_fft/2 + 1)` real spectrum.
Output length = `num_frames * (n_fft/2 + 1)`.

**Phase 5**: mel filterbank + mel-spectrogram + MFCC.

```rust
pub fn mel_filterbank(n_mels: u32, n_fft: u32, sample_rate: f32)
    -> Vec<f32>;  // [n_mels, n_fft/2 + 1] row-major

pub fn mel_spectrogram(signal: &[f32], n_fft: u32, hop_length: u32,
                       n_mels: u32, sample_rate: f32, window: WindowType)
    -> Result<Vec<f32>, StftError>;

pub fn mfcc(signal, n_fft, hop_length, n_mels, n_mfcc,
            sample_rate, window)
    -> Result<Vec<f32>, StftError>;
    // = DCT-II(log(mel_spectrogram))[:, :n_mfcc]
```

Composes `dsp-dct` (for the DCT-II final step in MFCC) with
this crate's `mel_spectrogram` + `log` for the canonical
speech feature pipeline.

**Phase 6**: matrix-ir-lowered STFT.

`build_stft_graph` + `stft_via_runtime` — emits a
`matrix_ir::Graph` that computes STFT through a series of
`fft_via_runtime` calls (one per frame) glued by Slice / Concat.
Lifts onto GPU once Metal / CUDA claim the relevant ops.

**Out of V1 scope**:

- **CQT** (Constant-Q Transform) — log-frequency spectrogram,
  used in music analysis.  Different math (geometric frequency
  spacing); future spec.
- **Wavelet transforms** — different theoretical foundation;
  DSP06 territory.
- **Centred / "reflect" padding modes** — Phase 4+ follow-up.
- **Real-time / streaming STFT** — current API is batch
  (whole signal at once).  A streaming `StftState::feed(...)`
  would let callers process audio block-by-block.

## Public API

Lives in a new crate **`dsp-stft`** depending on `dsp-fft`,
`dsp-dct` (for MFCC), and `dsp-filters` (for `WindowType`).

```rust
// Re-exported from dsp-filters::design — see DSP03 spec.
pub use dsp_filters::WindowType;

pub fn stft(signal: &[f32], n_fft: u32, hop_length: u32,
            window: WindowType)
    -> Result<Vec<f32>, StftError>;

pub fn istft(spectrogram: &[f32], n_fft: u32, hop_length: u32,
             window: WindowType, output_length: u32)
    -> Result<Vec<f32>, StftError>;

pub fn spectrogram(signal: &[f32], n_fft: u32, hop_length: u32,
                   window: WindowType)
    -> Result<Vec<f32>, StftError>;

pub fn log_spectrogram(signal: &[f32], n_fft: u32, hop_length: u32,
                       window: WindowType)
    -> Result<Vec<f32>, StftError>;

pub fn mel_filterbank(n_mels: u32, n_fft: u32, sample_rate: f32)
    -> Vec<f32>;

pub fn mel_spectrogram(signal: &[f32], n_fft: u32, hop_length: u32,
                       n_mels: u32, sample_rate: f32,
                       window: WindowType)
    -> Result<Vec<f32>, StftError>;

pub fn mfcc(signal: &[f32], n_fft: u32, hop_length: u32,
            n_mels: u32, n_mfcc: u32, sample_rate: f32,
            window: WindowType)
    -> Result<Vec<f32>, StftError>;

#[derive(Debug)]
pub enum StftError {
    EmptySignal,
    InvalidParam(String),    // n_fft = 0, hop = 0, etc.
    SignalTooShort(String),  // strict mode: signal < n_fft
    InvalidSpectrogram(String), // istft: shape mismatch
    Fft(String),             // wraps dsp_fft::FftError
}
```

## Numerical accuracy contract

- `istft(stft(x), …)` round-trips within `1e-4` relative
  tolerance for COLA-satisfying window/hop combinations,
  `N ≤ 64K`, f32 dtype.
- Spectrogram of a pure sinusoid concentrates energy in the
  bin closest to the sinusoid's frequency.
- Mel filterbank rows sum to ~1.0 (within normalisation
  convention).

## Phase plan

| Phase  | Lands                                                     | Risk |
| ------ | --------------------------------------------------------- | ---- |
| 0      | Spec (this document)                                      | Low. |
| 1+2    | Crate skeleton + scalar STFT (forward) + tests            | Low. |
| 3      | Scalar ISTFT (overlap-add reconstruction)                 | Medium — COLA normalisation. |
| 4      | Magnitude / power / log spectrogram helpers               | Low. |
| 5      | Mel filterbank + mel-spectrogram + MFCC                   | Medium — mel scale conventions. |
| 6      | Matrix-ir-lowered STFT via dsp-fft's matrix-ir path       | Medium. |

Phases 1+2 typically bundle.  Phase 5 may itself split into
5a (mel filterbank), 5b (mel spectrogram), 5c (MFCC) — each
builds on the previous.

## Dependencies

- `dsp-fft` — for the per-frame FFT (`fft_scalar` / `rfft_scalar`
  in Phases 2/3; `fft_via_runtime` in Phase 6).
- `dsp-dct` — for the DCT-II final step of MFCC.
- `dsp-filters` — for `WindowType` (already supports
  Rectangular / Hamming / Hann / Blackman; Kaiser is a
  pending DSP03 follow-up).
- `dsp-complex` — for the complex spectrum representation.
- `matrix-ir`, `matrix-runtime`, `matrix-cpu`, `compute-ir`,
  `executor-protocol` — Phase 6 only (same set dsp-fft and
  dsp-dct already pull in).

No FFI, no `unsafe`.

## Open questions

1. **Centred vs strict mode default.**  Librosa defaults to
   centred; scipy.signal.stft defaults to centred with reflect
   padding.  V1 ships strict mode only (simpler, no padding
   convention to choose); centred is a Phase 4+ extension
   gated behind an explicit `Padding` parameter.
2. **MDCT shape.**  Modified DCT for audio coding uses
   time-domain aliasing cancellation (TDAC) with 50% overlap
   Hann windows.  Currently deferred — when implemented, it'd
   likely live in `dsp-stft::mdct` rather than `dsp-dct`
   because the overlap-add framework belongs here.
3. **Streaming API.**  Real-time audio analysis wants
   `StftState::feed(&samples) -> Option<Frame>`.  Deferred
   until a real consumer asks; the batch API is the V1
   contract.
4. **Phase-vocoder / griffin-lim.**  Magnitude-only
   reconstruction (`magnitude_to_signal(mag) -> Vec<f32>`)
   is useful for source separation / TTS but iterative.
   Deferred to a future phase.
