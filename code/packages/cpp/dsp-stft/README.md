# dsp-stft (C++)

**Short-Time Fourier Transform, ISTFT, Spectrograms, Mel Filterbank & MFCC** — header-only, self-contained pure ISO C++17 (zero `<cmath>` / `-lm`) in namespace `ca::dsp_stft`.

Faithful C++17 implementation matching the Rust [`dsp-stft`](../../rust/dsp-stft) and TypeScript [`dsp-stft`](../../typescript/dsp-stft) contracts, bundling an internal $O(N \log N)$ radix-2 + Bluestein FFT and an internal $O(N \log N)$ Makhoul DCT-II.

## Features

- **Analysis windows**: `hann_window`, `hamming_window`, `blackman_window`, `rectangular_window`, `build_window`, `parse_window_type`.
- **STFT / ISTFT / COLA / Griffin-Lim**: `stft`, `istft`, `check_ola`, `griffin_lim`.
- **Spectrograms**: `magnitude_spectrogram`, `power_spectrogram`, `spectrogram`, `log_spectrogram`.
- **Speech features**: `hz_to_mel`, `mel_to_hz`, `mel_filterbank`, `mel_spectrogram`, `mfcc`.
- **VER15 & Zero-CVE hardened**: pre-flight bounds before element scans, checked `std::size_t` arithmetic, double intermediates with `|x| <= FLT_MAX` narrowing guards, and strictly iterative execution.

## Building

```sh
sh BUILD
```

On Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\run.ps1
```
