# @coding-adventures/dsp-dct

Discrete Cosine Transform (DCT-I..IV) and Discrete Sine Transform (DST-I..IV) with O(N log N) Makhoul FFT fast paths.

## Features

- **DCT-I..IV**: Complete support for Types I, II, III, and IV.
- **Fast Makhoul O(N log N) reductions**: DCT-II via FFT and DCT-III via IFFT using `@coding-adventures/dsp-fft`.
- **Normalization conventions**: Supports both unnormalized (`None`) and orthonormal (`Ortho`) scaling.
- **2D Transforms**: Row-column separable 2-D DCT/IDCT for image and spatial processing (`dct2d`, `idct2d`).
- **Discrete Sine Transform (DST-I..IV)**: Complete DST family.
- **VER15 Security Hardening**:
  - Pre-flight bounds checking (`MAX_DCT_LEN = 1,048,576`, `MAX_SAMPLES = 16,777,216`).
  - Fail-closed finite float validation (CWE-20).
  - Strictly iterative execution with zero recursion (CWE-674).
  - Zero regexes (CWE-1333).
  - Pure capabilities (`required_capabilities.json: []`).
