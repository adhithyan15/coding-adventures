# coding-adventures-dsp-filters

Pure Python FIR and IIR filters, filter design, biquads, and SOS cascades for the DSP layer (DSP03 / VER15).

Provides linear convolution FIR, FFT-accelerated FIR, direct-form-II transposed IIR, Butterworth and Chebyshev Type I designs, windowed-sinc FIR design, Audio EQ Cookbook biquads, and Second-Order Sections (SOS) cascade filtering.

## Features

- **FIR Filtering**: Direct linear convolution (`fir`) and FFT-based fast convolution (`fir_fft`).
- **IIR Filtering**: Direct-Form-II Transposed structure (`iir`) matching `scipy.signal.lfilter`.
- **Filter Design**:
  - Windowed-sinc lowpass, highpass, and bandpass designs (`design_low_pass`, `design_high_pass`, `design_band_pass`) with Rectangular, Hamming, Hann, and Blackman windows.
  - Butterworth lowpass design (`butterworth_lowpass`) for orders 1 and 2.
  - Chebyshev Type I lowpass design (`chebyshev1_lowpass`) for orders 1 and 2 with configurable passband ripple.
- **Biquads & SOS Cascade**: Audio EQ Cookbook biquads and cascade filtering (`Biquad`, `sos_cascade`, `sosfilt`).
- **Opt-in checked filtering**: `CheckedBiquad` and `checked_sosfilt` accept strict finite built-in numeric inputs, use direct coefficient division, and preserve filter state when a sample or block returns an error. Legacy `Biquad` and `sosfilt` behavior is unchanged.
- **Resource Exhaustion Defense (CWE-400)**: Pre-flight bounds checks (`MAX_SAMPLES = 16,777,216`, `MAX_TAPS = 1,048,576`, `MAX_ORDER = 1024`).
- **Finite Float Enforcement (CWE-20)**: Rejects `NaN`, `+Inf`, and `-Inf` fail-closed.
- **Recursion-Free (CWE-674)**: Strictly iterative execution (zero recursion).
- **Zero Regular Expressions (CWE-1333)**: Zero `re` imports or usage.
- **Pure Capabilities**: Declares zero OS capabilities (`[]`).
