# @coding-adventures/blas-library

Pluggable BLAS (Basic Linear Algebra Subprograms) library with 7 swappable backends -- Layer 3 of the accelerator computing stack.

## What is BLAS?

BLAS defines a standard API for fundamental linear algebra operations:

- **Level 1**: Vector-vector operations (SAXPY, DOT, NRM2, SCAL, ASUM, IAMAX, COPY, SWAP)
- **Level 2**: Matrix-vector operations (GEMV, GER)
- **Level 3**: Matrix-matrix operations (GEMM, SYMM, batched GEMM)
- **ML Extensions**: Activations (ReLU, GELU, sigmoid, tanh), softmax, layer norm, batch norm, conv2d, attention

## Available Backends

| Backend   | Name     | API Source   | Description                        |
|-----------|----------|--------------|------------------------------------|
| CpuBlas   | `cpu`    | Pure TS      | Reference implementation           |
| CudaBlas  | `cuda`   | CUDA Runtime | NVIDIA GPU (most popular for ML)   |
| MetalBlas | `metal`  | Metal API    | Apple Silicon unified memory       |
| VulkanBlas| `vulkan` | Vulkan API   | Maximum control, cross-platform    |
| OpenClBlas| `opencl` | OpenCL API   | Most portable (any vendor)         |
| WebGpuBlas| `webgpu` | WebGPU API   | Safe, browser-first                |
| OpenGlBlas| `opengl` | OpenGL API   | Legacy state machine               |

All GPU backends use the same computational logic (delegated to CpuBlas) but exercise vendor-specific memory management pipelines through the Layer 4 vendor-api-simulators.

## Quick Start

```typescript
import { createBlas, Matrix, Vector, Transpose } from "@coding-adventures/blas-library";

// Auto-select the best backend
const blas = createBlas("auto");

// Or pick a specific backend
const cpuBlas = createBlas("cpu");
const cudaBlas = createBlas("cuda");

// Matrix multiply: C = A * B
const A = new Matrix([1, 2, 3, 4], 2, 2);
const B = new Matrix([5, 6, 7, 8], 2, 2);
const C = new Matrix([0, 0, 0, 0], 2, 2);
const result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, A, B, 0.0, C);

// SAXPY: y = alpha * x + y
const x = new Vector([1, 2, 3], 3);
const y = new Vector([4, 5, 6], 3);
const saxpyResult = blas.saxpy(2.0, x, y);  // [6, 9, 12]
```

## Architecture

```
createBlas("auto") --> BackendRegistry --> [cuda, metal, vulkan, opencl, webgpu, opengl, cpu]
                                                |
                                           GpuBlasBase
                                           (template method pattern)
                                                |
                                          _upload() / _download() / _free()
                                                |
                                    Layer 4: vendor-api-simulators
```

## How It Fits in the Stack

- **Layer 1-2**: Logic gates, arithmetic
- **Layer 3**: This library (BLAS operations)
- **Layer 4**: Vendor API simulators (CUDA, Metal, Vulkan, etc.)
- **Layer 5**: Vulkan-inspired compute runtime

## Running Tests

```bash
npx vitest run              # Run all tests
npx vitest run --coverage   # Run with coverage report
```
