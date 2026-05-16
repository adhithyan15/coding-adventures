# Changelog — dsp-stft

## 0.2.0 — 2026-05-16

### Added — DSP05 Phase 3 + 4 (ISTFT + spectrogram helpers)

Closes the analysis/synthesis loop and adds the magnitude /
log helpers that downstream audio code actually wants.

#### Public API

```rust
pub fn istft(spectrogram, n_fft, hop_length, window, output_length)
    -> Result<Vec<f32>, StftError>;

pub fn spectrogram(signal, n_fft, hop_length, window)
    -> Result<Vec<f32>, StftError>;

pub fn log_spectrogram(signal, n_fft, hop_length, window)
    -> Result<Vec<f32>, StftError>;
```

All re-exported at the crate root.

#### Algorithm — `istft` (overlap-add)

For each frame `m`, irfft the spectrum to length-`n_fft` time
domain, multiply by the synthesis window, add into the output
buffer at position `m · hop_length`.  Also accumulate the
sum-of-squared-windows at each position.  Divide the output
by that running norm (skipping samples where the norm is
near-zero).

Under COLA-satisfying window/hop choices (Hann at
`hop = n_fft / 2` is canonical), `istft(stft(x))` recovers
`x` within `1e-3` relative tolerance in the central region
(edges are subject to the usual transient boundary effects).

#### `spectrogram` / `log_spectrogram`

- `spectrogram(signal, ...)` — returns `|STFT|²` flattened
  `[num_frames, n_fft/2 + 1]`.  Half the size of the complex
  spectrogram (one magnitude lane instead of two `[re, im]`
  lanes).  Non-negative by construction.
- `log_spectrogram(signal, ...)` — `log(|STFT|² + ε)` with
  `ε = 1e-10`.  Always finite (no `-∞` at silent bins).  The
  canonical input feature for plotting and ML pipelines.

#### New unit tests — 14

`inverse` module (8):
- 5 error paths: empty spectrogram, n_fft=0, hop=0,
  output_length=0, misaligned spectrogram length.
- COLA round-trip recovers a signal within 1e-3 (Hann/hop=n_fft/2).
- Output length matches request.
- Constant signal round-trips to the same constant in central region.

`spectrogram` module (6):
- spectrogram non-negative.
- length matches `num_frames × bins`.
- zero signal → all-zero spectrogram.
- log_spectrogram finite everywhere.
- log_spectrogram of zero signal ≈ log(ε) ≈ -23.0259.
- log_spectrogram length matches spectrogram length.

All 25 unit tests + 1 doctest pass (11 stft + 8 inverse + 6 spectrogram).

### What this phase does NOT include

- Phase 5: mel filterbank + mel_spectrogram + MFCC.
- Phase 6: matrix-ir-lowered stft.
- Centred-padding mode.
- Streaming / real-time API.

## 0.1.0 — 2026-05-16

### Added — DSP05 Phase 1 + 2 (crate skeleton + scalar STFT forward)

Initial release.  The scalar Short-Time Fourier Transform —
sliding-window FFT that's the foundation of every modern
audio / speech / music analysis pipeline.

#### Public API

```rust
pub use dsp_filters::WindowType;

pub fn stft(signal: &[f32], n_fft: u32, hop_length: u32,
            window: WindowType)
    -> Result<Vec<f32>, StftError>;

pub enum StftError {
    EmptySignal,
    InvalidParam(String),
    SignalTooShort(String),
    InvalidSpectrogram(String),  // reserved for istft (Phase 3)
    Fft(String),
}
```

#### Algorithm

Strict-mode framing.  For each frame `m`:

1. Extract `signal[m * hop_length .. m * hop_length + n_fft]`.
2. Multiply by analysis window built from `WindowType`
   (Rectangular / Hamming / Hann / Blackman — same formulas
   as `dsp_filters::design`).
3. Run `dsp_fft::rfft_scalar` on the windowed frame.
4. Append the resulting `n_fft/2 + 1` complex bins to the
   output buffer.

Output layout: row-major `[num_frames, n_fft/2 + 1, 2]` —
interleaved `[re, im]` per bin — flattened to a `Vec<f32>`
of length `num_frames * (n_fft/2 + 1) * 2`.

`num_frames = 1 + (N - n_fft) / hop_length`.

#### Validation

- `signal.is_empty()` → `EmptySignal`.
- `n_fft == 0` or `hop_length == 0` → `InvalidParam`.
- `signal.len() < n_fft` (strict mode) → `SignalTooShort`.

#### Tests — 11

Error paths (4):
- `stft_rejects_empty_signal`
- `stft_rejects_zero_n_fft`
- `stft_rejects_zero_hop`
- `stft_rejects_signal_shorter_than_n_fft`

Output contract (1):
- `stft_output_length_matches_num_frames`

Closed-form / known vectors (4):
- `stft_of_constant_signal_concentrates_at_dc` — constant
  signal under Hann window: each frame's bin 0 = sum of
  windowed frame; bins 1..N/2+1 ≈ 0 (Hann is band-limited).
- `stft_of_pure_sinusoid_peaks_at_expected_bin` — 440 Hz
  sinusoid at 44.1 kHz, n_fft=1024: peaks at bin 10 (closest
  to 440 * 1024 / 44100 ≈ 10.2).
- `stft_with_rectangular_window_matches_per_frame_rfft` —
  cross-validates the framing/striding logic against direct
  calls to `dsp_fft::rfft_scalar`.
- `stft_with_hop_equal_to_n_fft_gives_disjoint_frames` —
  hop_length = n_fft means no overlap, frame count =
  N / n_fft.

Numerical sanity (2):
- `stft_with_hann_window_attenuates_high_frequencies` — a
  high-frequency tone (close to Nyquist) through a Hann
  window has lower bin magnitude than the same tone through
  a Rectangular window (Hann's smooth taper reduces spectral
  leakage but also attenuates the main lobe a bit).
- `stft_num_frames_matches_formula` — explicit check for
  several (N, n_fft, hop_length) combinations.

#### Dependencies

- `dsp-fft` — for `rfft_scalar` (per-frame FFT).
- `dsp-filters` — for `WindowType` (the analysis window enum).
- `dsp-complex` — reserved for Phase 4+ when we expose
  `ComplexTensor` views over the spectrogram.

No FFI, no `unsafe`, no external crates.

#### What this phase does NOT include

- Phase 3: `istft` (inverse STFT via overlap-add).
- Phase 4: `spectrogram` / `log_spectrogram` helpers.
- Phase 5: `mel_filterbank` + `mel_spectrogram` + `mfcc`.
- Phase 6: matrix-ir-lowered `stft`.
- Centred-padding mode (V1 ships strict only).
- Streaming / real-time API.
