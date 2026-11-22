# dsp-stft (C)

**Short-Time Fourier Transform, ISTFT, Spectrograms, Mel Filterbank & MFCC** — self-contained pure ISO C17 (zero `<math.h>` / `-lm`).

Faithful C17 implementation matching the Rust [`dsp-stft`](../../rust/dsp-stft) and TypeScript [`dsp-stft`](../../typescript/dsp-stft) contracts, bundling an internal $O(N \log N)$ radix-2 + Bluestein FFT and an internal $O(N \log N)$ Makhoul DCT-II.

## Features

- **Analysis windows**: `stft_hann_window`, `stft_hamming_window`, `stft_blackman_window`, `stft_rectangular_window`, `stft_build_window`, `stft_parse_window`.
- **STFT / ISTFT / COLA / Griffin-Lim**: `stft_forward`, `stft_istft`, `stft_check_ola` (NOLA/COLA check), `stft_griffin_lim`.
- **Spectrograms**: `stft_magnitude_spectrogram`, `stft_power_spectrogram`, `stft_spectrogram`, `stft_log_spectrogram`, `stft_log_spectrogram_eps`.
- **Speech features**: `stft_hz_to_mel`, `stft_mel_to_hz`, `stft_mel_filterbank`, `stft_mel_spectrogram`, `stft_mfcc`.
- **VER15 & Zero-CVE hardened**: pre-flight bounds before element scans, checked `size_t` arithmetic, double intermediates with `|x| <= FLT_MAX` narrowing guards, and idempotent volatile zero-on-free (`stft_free_buf`).

## Building

```sh
sh BUILD
```

On Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\run.ps1
```
