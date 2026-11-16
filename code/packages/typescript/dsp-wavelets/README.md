# @coding-adventures/dsp-wavelets

Discrete and Continuous Wavelet Transforms for the DSP layer.

## Features

- 1-D Discrete Wavelet Transform (`dwt`, `idwt`, `dwt1d`, `idwt1d`) via the Mallat pyramid algorithm.
- 2-D Discrete Wavelet Transform (`dwt2d`, `idwt2d`) for quad-tree sub-band decomposition of images.
- Multi-level decomposition and reconstruction (`wavedec`, `waverec`).
- Stationary / Undecimated Wavelet Transform (`swt`, `iswt`).
- Continuous Wavelet Transform (`cwt`) supporting Morlet and Ricker (Mexican Hat) wavelets.
- Wavelet filter banks: Haar, Daubechies (db1..8), Symlets (sym2, sym4), Coiflets (coif1), Biorthogonal (bior2.2 / 5.3).
- Boundary modes: Zero, Replicate, Reflect, Symmetric, Periodic.
- Hardened with VER15 CWE/CVE proofs (CWE-20, CWE-190, CWE-400, CWE-674, CWE-1333). Zero ambient capabilities.
