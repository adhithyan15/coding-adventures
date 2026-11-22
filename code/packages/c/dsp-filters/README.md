# dsp-filters (C)

**FIR, IIR, Biquad, SOS, and Butterworth Filters** — pure ISO C17 (zero `<math.h>`, zero `-lm`).

## What it does

- **FIR filtering & design**: direct linear convolution (`dsp_fir`), FFT fast convolution (`dsp_fir_fft`),
  and windowed-sinc `dsp_design_{low_pass,high_pass,band_pass,band_stop}` with Rectangular, Hamming,
  Hann, and Blackman windows.
- **Direct & zero-phase IIR**: Direct-Form-II Transposed (`dsp_iir`, `dsp_filtfilt`) and alias-safe
  `*_into` variants (`out == signal`).
- **Checked Biquad & SOS**: failure-atomic `dsp_biquad_t` with Audio EQ Cookbook constructors
  (`lowpass`, `highpass`, `bandpass`, `notch`, `allpass`, `peak`), plus `dsp_sosfilt` and
  `dsp_sosfiltfilt`.
- **Butterworth design**: bilinear-transformed `dsp_butterworth_sos` (orders 1..64) and
  `dsp_butterworth_{lowpass,highpass,bandpass,bandstop}` (orders 1..2).

## Building

```sh
sh BUILD
```
