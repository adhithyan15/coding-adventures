# @coding-adventures/dsp-filters

Digital FIR and IIR filters and classical filter design helpers for the DSP layer.

## Features

- **Second-Order Sections (Biquad)**: Audio EQ Cookbook biquad filters (`lowpass`, `highpass`, `bandpass`, `notch`, `allpass`, `peak`) with Direct-Form II Transposed state processing.
- **IIR Filtering**: Direct-Form II Transposed structure matching `scipy.signal.lfilter`, and `filtfilt` zero-phase forward-backward filtering.
- **SOS Cascade**: Biquad cascade processing matching `scipy.signal.sosfilt`.
- **Opt-in checked filtering**: `CheckedBiquad` and `checkedSosfilt` accept primitive finite numbers, normalize by direct division, and preserve filter state when a sample or complete block returns an error. Legacy `Biquad` and `sosfilt` behavior is unchanged.
- **FIR Filtering**: Direct linear convolution (`fir`) and FFT-accelerated fast convolution (`firFft` / `fir_fft`).
- **Filter Design**:
  - `firwin`: Windowed-sinc FIR design (lowpass, highpass, bandpass, bandstop) matching `scipy.signal.firwin`.
  - Bilinear-transformed Butterworth IIR filters (`butterworthLowpass`, `butterworthHighpass`, `butterworthBandpass`).
  - Bilinear-transformed Chebyshev Type I IIR filters (`chebyshev1Lowpass`).
  - Windowed-sinc primitives (`designLowPass`, `designHighPass`, `designBandPass`).
- **VER15 Security Hardening**:
  - Pre-flight bounds checking (`MAX_FILTER_TAPS = 65,536`, `MAX_SIGNAL_LEN = 16,777,216`, `MAX_ORDER = 1024`).
  - Fail-closed finite float validation (CWE-20).
  - Strictly iterative execution with zero recursion across all filter loops (CWE-674).
  - Zero regexes (CWE-1333).
  - Pure capabilities (`required_capabilities.json: []`).
