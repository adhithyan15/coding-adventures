# coding-adventures-dsp-dct

Pure Python Discrete Cosine and Sine Transforms (DCT/DST) for the DSP layer (DSP02 / VER15).

Provides fast $O(N \log N)$ DCT-II / DCT-III transforms via `dsp-fft`, textbook DCT-I..IV and DST-I..IV transforms, and 2-D transforms for JPEG / image processing.

## Features

- **DCT-I through DCT-IV**: Complete support for all four classical cosine transforms with `None` and `Ortho` normalizations.
- **Fast $O(N \log N)$ DCT-II / DCT-III**: Reduced to FFT / IFFT using Makhoul's reduction.
- **DST-I through DST-IV**: Complete support for all four classical sine transforms.
- **2-D DCT and IDCT**: Row-then-column factorization on flattened $H \times W$ buffers.
- **Resource Exhaustion Defense (CWE-400)**: Pre-flight bounds checks (`MAX_DCT_LEN = 1,048,576`, `MAX_SAMPLES = 16,777,216`).
- **Finite Float Enforcement (CWE-20)**: Rejects `NaN`, `+Inf`, and `-Inf` fail-closed.
- **Recursion-Free (CWE-674)**: Strictly iterative execution.
- **Zero Regular Expressions (CWE-1333)**: Zero `re` imports or usage.
- **Pure Capabilities**: Declares zero OS capabilities (`[]`).
