# @coding-adventures/dsp-conv

Same-size and discrete convolution and image filters for the DSP layer.

## Features

- 1-D same-size convolution (`conv1d`) and discrete convolution (`convolve`) with Full, Same, and Valid shape modes.
- Four boundary extension modes (`Zero`, `Replicate`, `Reflect`, `Wrap`).
- Discrete cross-correlation (`correlate`).
- O(N log N) FFT-based convolution (`fftConvolve` / `fft_convolve`) via `@coding-adventures/dsp-fft`.
- Overlap-add block convolution (`overlapAdd` / `overlap_add`) for streaming / long signals.
- Standard image filter kernels: Gaussian blur (`gaussianBlurKernel`), box blur (`boxBlurKernel`), Sobel edge detectors (`sobelXKernel`, `sobelYKernel`), discrete Laplacian (`laplacianKernel`), and unsharp mask (`sharpenKernel`).
- 2-D convolution (`conv2d`, `convolve2d`) and fast separable 2-D filtering (`sepConv2d`).
- Hardened with VER15 CWE/CVE proofs (CWE-20, CWE-190, CWE-400, CWE-674, CWE-1333). Zero ambient capabilities.
