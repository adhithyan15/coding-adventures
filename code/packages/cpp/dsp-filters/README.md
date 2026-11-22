# dsp-filters (C++)

**FIR, IIR, Biquad, SOS, and Butterworth Filters** — header-only, pure ISO C++17 in namespace `ca::dsp_filters`.

## What it does

- **FIR filtering & design**: `fir`, `fir_fft`, and `design_{low_pass,high_pass,band_pass,band_stop}`.
- **Direct & zero-phase IIR**: `iir`, `filtfilt`, and alias-safe `*_into`.
- **Checked Biquad & SOS**: failure-atomic `Biquad` with Audio EQ Cookbook constructors, `sosfilt`, and `sosfiltfilt`.
- **Butterworth design**: `butterworth_sos` (LP/HP/BP/BS, orders 1..64) and `butterworth_{lowpass,highpass,bandpass,bandstop}` (orders 1..2).

## Building

```sh
sh BUILD
```
