# @coding-adventures/dsp-fft

Iterative Fast Fourier Transform for the DSP layer.

## Features

- **Radix-2 Cooley-Tukey FFT**: Strictly iterative in-place butterflies and bit-reversal for power-of-two lengths.
- **Bluestein's Algorithm**: Arbitrary-length FFT via chirp Z-transform convolution.
- **Real-input Transforms**: `rfft` and `irfft` exploiting conjugate symmetry to halve compute and memory.
- **Window Functions**: `hann`, `hamming`, `blackman`, `kaiser` window generators.
- **VER15 Security Hardening**:
  - Pre-flight bounds checking (`MAX_FFT_LEN = 1,048,576`, `MAX_SAMPLES = 16,777,216`) enforcing CWE-400 limits before allocation.
  - Fail-closed finite float validation (CWE-20).
  - Strictly iterative execution with zero recursion across all butterfly and convolution loops (CWE-674).
  - Zero regexes (CWE-1333).
  - Pure capabilities (`required_capabilities.json: []`).
