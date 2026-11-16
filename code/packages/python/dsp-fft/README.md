# coding-adventures-dsp-fft

Pure Python Cooley-Tukey radix-2 and Bluestein FFT for the DSP layer (DSP01 / VER15).

Provides iterative decimation-in-time radix-2 FFT and arbitrary-N Bluestein FFT algorithms operating on interleaved `[re, im]` float sequences and `dsp_complex.ComplexTensor`.

## Features

- **Radix-2 FFT/IFFT**: Strictly iterative Cooley-Tukey butterfly network for power-of-two lengths.
- **Bluestein FFT/IFFT**: Chirp Z-transform algorithm recasting arbitrary-$N$ DFT as linear convolution.
- **Half-Spectrum RFFT/IRFFT**: Real-input FFT exploiting conjugate symmetry, returning $\lfloor N/2 \rfloor + 1$ complex bins.
- **Resource Exhaustion Defense (CWE-400)**: Pre-flight bounds checks (`MAX_FFT_LEN = 1,048,576`, `MAX_SAMPLES = 16,777,216`).
- **Finite Float Enforcement (CWE-20)**: Rejects `NaN`, `+Inf`, and `-Inf` fail-closed.
- **Recursion-Free (CWE-674)**: Strictly iterative execution (zero recursive calls).
- **Zero Regular Expressions (CWE-1333)**: Zero `re` imports or usage.
- **Pure Capabilities**: Declares zero OS capabilities (`[]`).
