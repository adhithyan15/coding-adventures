# dsp-stft

Short-Time Fourier Transform (STFT) for the DSP layer.  As of
**0.4.0 (DSP05 Phase 6)** the crate ships the complete DSP05
phase plan — the scalar reference *and* a matrix-IR-lowered
execution path that lifts onto GPU once Metal / CUDA claim the
relevant ops:

- **`stft`** — forward (sliding-window FFT, scalar).
- **`istft`** — inverse via overlap-add reconstruction (COLA).
- **`spectrogram`** — `|STFT|²` (magnitude squared).
- **`log_spectrogram`** — `log(|STFT|² + ε)`.
- **`mel_filterbank`** — `[n_mels, n_fft/2 + 1]` triangular
  filters on the mel scale (HTK convention), row-normalised.
- **`mel_spectrogram`** — `mel_filterbank @ |STFT|²`.
- **`mfcc`** — `DCT-II_ortho(log(mel_spectrogram + ε))[:, :n_mfcc]`.
- **`build_stft_graph`** — emits a `matrix_ir::Graph` that
  computes STFT through generic tensor ops (Slice, Mul,
  Concat, Reshape, Const).
- **`stft_via_runtime`** — end-to-end matrix-IR-lowered
  execution; numerically identical to scalar `stft` within
  `~1e-5`.

The STFT is the workhorse primitive for time-frequency
analysis of audio, speech, and music.  Plain DFT collapses a
whole signal into one spectrum; STFT slides a windowed FFT
over the signal and emits a 2-D `[time, frequency]` matrix —
the spectrogram representation everyone uses for speech
recognition, source separation, audio coding, MFCCs, etc.

```rust
use dsp_stft::{stft, WindowType};

let signal: Vec<f32> = (0..16_384)
    .map(|n| (2.0 * std::f32::consts::PI * 440.0 * (n as f32) / 44_100.0).sin())
    .collect();

// 1024-sample FFT, 50% overlap, Hann window.
let spec = stft(&signal, 1024, 512, WindowType::Hann).unwrap();
// Layout: [num_frames, 513, 2] interleaved [re, im], flattened to Vec<f32>.
```

## Algorithm

For each frame `m ∈ [0, num_frames)` where
`num_frames = 1 + (N - n_fft) / hop_length`:

1. Extract frame: `signal[m * hop_length .. m * hop_length + n_fft]`.
2. Multiply by analysis window `w[n]`.
3. Run `rfft` on the windowed frame → length `n_fft/2 + 1`
   complex spectrum.
4. Append to output (row-major
   `[num_frames, n_fft/2 + 1, 2]` interleaved `[re, im]`).

V1 uses **strict framing**: no centred padding, only frames
that fit entirely inside the signal.  Phase 4+ will add a
centred-padding mode matching librosa / scipy defaults.

## Public API

```rust
pub use dsp_filters::WindowType;

pub fn stft(signal: &[f32], n_fft: u32, hop_length: u32,
            window: WindowType)
    -> Result<Vec<f32>, StftError>;

pub enum StftError {
    EmptySignal,
    InvalidParam(String),
    SignalTooShort(String),
    InvalidSpectrogram(String),
    Fft(String),
}
```

## Phase scope

| Phase  | Lands                                                | Status |
| ------ | ---------------------------------------------------- | ------ |
| 0      | Spec (`code/specs/DSP05-stft.md`)                    | landed |
| **1+2** | **Crate skeleton + scalar `stft` (forward)**        | **this PR (0.1.0)** |
| 3      | Scalar `istft` (overlap-add reconstruction)          | landed (0.2.0) |
| 4      | `spectrogram` / `log_spectrogram` helpers            | landed (0.2.0) |
| 5      | `mel_filterbank` + `mel_spectrogram` + `mfcc`        | landed (0.3.0) |
| **6**  | **Matrix-ir-lowered STFT via dsp-fft's matrix-ir path** | **this PR (0.4.0)** |

## Tests

`cargo test -p dsp-stft` exercises:

- Error paths: empty signal, `n_fft=0`, `hop_length=0`, signal
  shorter than `n_fft`.
- Output length contract:
  `output.len() == num_frames * (n_fft/2 + 1) * 2`.
- DC sinusoid concentrates all energy in bin 0.
- Pure sinusoid peaks in the bin closest to its frequency.
- Rectangular window matches plain `rfft` of each frame
  (cross-validates the framing logic).
