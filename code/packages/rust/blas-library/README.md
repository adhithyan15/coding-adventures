# blas-library

Pluggable BLAS (Basic Linear Algebra Subprograms) library with 7 swappable backends.

## What It Does

This crate provides a complete BLAS implementation covering:

- **Level 1** (vector-vector): SAXPY, SDOT, SNRM2, SASUM, ISAMAX, SSCAL, SCOPY, SSWAP
- **Level 2** (matrix-vector): SGEMV, SGER
- **Level 3** (matrix-matrix): SGEMM, SSYMM, SGEMM_BATCHED
- **ML Extensions**: ReLU, GELU, Sigmoid, Tanh, Softmax, LayerNorm, BatchNorm, Conv2D, Attention

## The 7 Backends

| Backend    | Hardware          | Key Feature                |
|------------|-------------------|----------------------------|
| **CPU**    | Any CPU           | Pure Rust, reference impl  |
| **CUDA**   | NVIDIA GPUs       | cuBLAS-style memory model  |
| **Metal**  | Apple Silicon     | Unified memory model       |
| **OpenCL** | Any GPU/CPU/FPGA  | Portable, event-driven     |
| **Vulkan** | Cross-platform    | Explicit memory management |
| **WebGPU** | Browsers          | Safe, validated API        |
| **OpenGL** | Legacy GPUs       | State machine model        |

All GPU backends exercise the full memory pipeline (allocate, upload, compute, download, free) via vendor API simulators from Layer 4.

## How It Fits in the Stack

```
Layer 2: blas-library (this crate)
    |
Layer 3: vendor-api-simulators (CUDA, Metal, OpenCL, Vulkan, WebGPU, OpenGL)
    |
Layer 5: compute-runtime (Vulkan-inspired runtime)
    |
Layer 6: device-simulator (GPU hardware simulation)
```

## Quick Start

```rust
use blas_library::{CpuBlas, Vector, Matrix, Transpose};
use blas_library::traits::BlasBackend;

let blas = CpuBlas;

// SAXPY: y = 2*x + y
let x = Vector::new(vec![1.0, 2.0, 3.0]);
let y = Vector::new(vec![4.0, 5.0, 6.0]);
let result = blas.saxpy(2.0, &x, &y).unwrap();
assert_eq!(result.data(), &[6.0, 9.0, 12.0]);

// GEMM: C = A * B
let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2);
let c = Matrix::zeros(2, 2);
let result = blas.sgemm(
    Transpose::NoTrans, Transpose::NoTrans,
    1.0, &a, &b, 0.0, &c,
).unwrap();
assert_eq!(result.data(), &[19.0, 22.0, 43.0, 50.0]);
```

## Backend Selection

```rust
use blas_library::BackendRegistry;

let registry = BackendRegistry::with_defaults();

// Auto-detect the best available backend
let best = registry.get_best().unwrap();
println!("Using: {} ({})", best.name(), best.device_name());

// Or request a specific one
let cpu = registry.get("cpu").unwrap();
```

## Building and Testing

```bash
cargo test -p blas-library
```

## Test Coverage

301 tests covering:
- All Level 1, 2, 3 BLAS operations
- All ML extensions (activations, normalization, convolution, attention)
- All 7 backends (creation, device names, GPU pipeline)
- Cross-backend consistency (all backends produce identical results)
- Edge cases (empty vectors, 1x1 matrices, special float values, numerical stability)
- Error handling (dimension mismatches across all operations and backends)
