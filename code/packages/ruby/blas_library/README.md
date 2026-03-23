# coding_adventures_blas_library

A complete BLAS (Basic Linear Algebra Subprograms) library with seven interchangeable backends, built on simulated GPU hardware. Layer 6 of the accelerator computing stack.

## Overview

This package provides the standard BLAS API (Level 1, 2, 3) plus ML extensions (activation functions, normalization, convolution, attention) across seven backends:

| Backend | Class | GPU API | Vendor |
|---------|-------|---------|--------|
| CPU | `CpuBlas` | None (pure Ruby) | Universal |
| CUDA | `CudaBlas` | CUDARuntime | NVIDIA |
| Metal | `MetalBlas` | MTLDevice | Apple |
| OpenCL | `OpenClBlas` | CLContext | Cross-platform |
| Vulkan | `VulkanBlas` | VkDevice | Cross-platform |
| WebGPU | `WebGpuBlas` | GPUDevice | Browser-safe |
| OpenGL | `OpenGlBlas` | GLContext | Legacy |

All backends produce **identical results** -- the GPU backends exercise the full GPU memory pipeline (allocate, upload, compute, download, free) while delegating arithmetic to the CPU reference.

## Quick Start

```ruby
require "coding_adventures_blas_library"
include CodingAdventures::BlasLibrary

# Create a backend
blas = create_blas("auto")   # Best available
blas = create_blas("cpu")    # Specific backend

# Vector operations (Level 1)
x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
y = Vector.new(data: [4.0, 5.0, 6.0], size: 3)
result = blas.saxpy(2.0, x, y)   # => [6.0, 9.0, 12.0]
dot = blas.sdot(x, y)            # => 32.0
norm = blas.snrm2(x)             # => 3.742...

# Matrix operations (Level 3)
a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
b = Matrix.new(data: [5.0, 6.0, 7.0, 8.0], rows: 2, cols: 2)
c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
result = blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)

# ML extensions
activated = blas.relu(a)
probs = blas.softmax(a)
output = blas.attention(q, k, v)
```

## BLAS Operations

### Level 1 (Vector-Vector, O(n))
- `saxpy(alpha, x, y)` -- y = alpha * x + y
- `sdot(x, y)` -- dot product
- `snrm2(x)` -- Euclidean norm
- `sscal(alpha, x)` -- scale vector
- `sasum(x)` -- absolute sum (L1 norm)
- `isamax(x)` -- index of max absolute value
- `scopy(x)` -- deep copy
- `sswap(x, y)` -- swap two vectors

### Level 2 (Matrix-Vector, O(n^2))
- `sgemv(trans, alpha, a, x, beta, y)` -- matrix-vector multiply
- `sger(alpha, x, y, a)` -- outer product (rank-1 update)

### Level 3 (Matrix-Matrix, O(n^3))
- `sgemm(trans_a, trans_b, alpha, a, b, beta, c)` -- matrix multiply
- `ssymm(side, alpha, a, b, beta, c)` -- symmetric matrix multiply
- `sgemm_batched(...)` -- batched matrix multiply

### ML Extensions
- `relu(x)`, `gelu(x)`, `sigmoid(x)`, `tanh_activation(x)` -- activations
- `softmax(x, axis:)` -- probability distribution
- `layer_norm(x, gamma, beta, eps:)` -- layer normalization
- `batch_norm(x, gamma, beta, rm, rv, eps:, training:)` -- batch normalization
- `conv2d(input, weight, bias:, stride:, padding:)` -- 2D convolution
- `attention(q, k, v, mask:, scale:)` -- scaled dot-product attention

## Architecture

```
Layer 6 (this package):  BLAS operations
Layer 5:                 Vendor API simulators (CUDA, Metal, etc.)
Layer 4:                 Compute runtime (Vulkan-inspired)
Layers 1-3:              Hardware simulation (logic gates -> ALU -> GPU)
```

## Running Tests

```bash
bundle install
bundle exec rake test
```
