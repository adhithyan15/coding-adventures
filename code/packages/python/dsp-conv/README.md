# coding-adventures-dsp-conv

Pure Python same-size and discrete convolution, correlation, 2-D convolution, FFT convolution, overlap-add, and image filters for the DSP layer.

## Features

- **1-D Convolution & Correlation**: `convolve`, `correlate`, and `conv1d` supporting `Full`, `Same`, `Valid` modes and `Zero`, `Replicate`, `Reflect`, `Wrap` boundary extensions.
- **Fast FFT Convolution**: `fft_convolve` using radix-2 and Bluestein FFTs from `dsp-fft`.
- **Block Processing**: `overlap_add` streaming convolution for large signals.
- **2-D Convolution**: `conv2d` and `convolve2d` for row-major and nested matrix filtering.
- **Separable 2-D Convolution**: `sep_conv2d` row-then-column fast path for separable kernels.
- **Filter Design Helpers**: `gaussian_blur_kernel`, `sobel_x_kernel`, `sobel_y_kernel`, `box_blur_kernel`, `laplacian_kernel`, `sharpen_kernel`.
- **VER15 Security Hardening**: Pre-flight CWE-400 bounds (`MAX_SAMPLES`, `MAX_KERNEL_LEN`), CWE-20 finite-float validation, CWE-674 iterative execution (zero recursion), CWE-1333 zero regex imports, and pure capability sandbox compliance.
