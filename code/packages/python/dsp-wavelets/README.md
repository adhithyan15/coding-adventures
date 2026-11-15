# coding-adventures-dsp-wavelets

Pure Python Discrete, Stationary, and Continuous Wavelet Transforms (DWT, IDWT, SWT, CWT) for the DSP layer.

## Features

- **Wavelet Families**: Haar, Daubechies (Db1-Db8), Symlets (Sym2-Sym8), Coiflets (Coif1-Coif3), Biorthogonal (Bior2.2 / 5.3), Morlet, and Ricker / Mexican Hat.
- **1-D DWT and Inverse DWT**: Single-level `dwt` / `idwt` and multi-level Mallat pyramid `dwt_1d` / `idwt_1d`.
- **Wavelet Decomposition & Reconstruction**: `wavedec` and `waverec` multi-resolution analysis.
- **Boundary Extension Modes**: `Zero`, `Replicate`, `Reflect`, `Symmetric`, and `Periodic`.
- **Stationary Wavelet Transform (SWT)**: Undecimated translation-invariant wavelet decomposition `swt` and exact reconstruction `iswt`.
- **Continuous Wavelet Transform (CWT)**: Multi-scale time-frequency scalogram analysis with Morlet and Ricker wavelets.
- **2-D DWT**: Image quad-tree decomposition `dwt_2d` and synthesis `idwt_2d`.
- **VER15 Security Hardening**: Pre-flight bounds (`MAX_SAMPLES`, `MAX_WAVELET_LEVELS = 32`, `MAX_SCALES`), CWE-20 finite-float checks, CWE-674 iterative execution, CWE-1333 zero regex imports, and pure capability sandbox compliance.
