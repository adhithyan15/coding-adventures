# BLAS Library (Go)

Layer 3 of the accelerator computing stack -- a pluggable BLAS (Basic Linear Algebra Subprograms) library with seven interchangeable backend implementations.

## What is BLAS?

BLAS (Basic Linear Algebra Subprograms) is a specification for standard linear algebra operations. Published in 1979, it defines vector and matrix operations at three levels:

| Level | Year | Operations | Complexity | Examples |
|-------|------|-----------|------------|----------|
| 1 | 1979 | Vector-Vector | O(n) | SAXPY, DOT, NRM2, SCAL |
| 2 | 1988 | Matrix-Vector | O(n^2) | GEMV, GER |
| 3 | 1990 | Matrix-Matrix | O(n^3) | GEMM, SYMM, Batched GEMM |
| ML | 2024 | Extensions | varies | ReLU, Softmax, Attention, Conv2d |

## The Seven Backends

All seven backends implement the same `BlasBackend` and `MlBlasBackend` interfaces and produce identical results (within floating-point tolerance):

| Backend | Wraps | Vendor | Key Feature |
|---------|-------|--------|-------------|
| `CpuBlas` | Pure Go loops | Any | Universal fallback, reference implementation |
| `CudaBlas` | CUDA Runtime | NVIDIA | cudaMalloc/cudaMemcpy pipeline |
| `MetalBlas` | MTLDevice | Apple | Unified memory (no host-device copies) |
| `VulkanBlas` | VkInstance | Any | Maximum explicit control |
| `OpenClBlas` | CLContext | Any | Event-based dependencies, most portable |
| `WebGpuBlas` | GPUDevice | Any | Safe browser-first API |
| `OpenGlBlas` | GLContext | Any | Legacy state machine model |

## Architecture

```
blaslibrary (package)
  types.go       -- Vector, Matrix, enums (StorageOrder, Transpose, Side)
  blas.go        -- BlasBackend and MlBlasBackend interfaces
  registry.go    -- BackendRegistry with auto-detection

backends (sub-package)
  cpu.go         -- Pure Go reference implementation
  gpu_base.go    -- Shared GPU template (upload/compute/download)
  cuda.go        -- CUDA backend
  metal.go       -- Metal backend
  opencl.go      -- OpenCL backend
  vulkan.go      -- Vulkan backend
  webgpu.go      -- WebGPU backend
  opengl.go      -- OpenGL backend
  init.go        -- Registers all backends with GlobalRegistry
```

## Usage

```go
import (
    blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
    _ "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library/backends"
)

// Auto-detect the best available backend
backend, _ := blas.CreateBlas("auto")

// Or pick a specific backend
backend, _ = blas.CreateBlas("cuda")

// SAXPY: y = alpha * x + y
x := blas.NewVector([]float32{1, 2, 3})
y := blas.NewVector([]float32{4, 5, 6})
result, _ := backend.Saxpy(2.0, x, y) // [6, 9, 12]

// GEMM: C = alpha * A * B + beta * C
A, _ := blas.NewMatrix([]float32{1, 2, 3, 4}, 2, 2)
B, _ := blas.NewMatrix([]float32{5, 6, 7, 8}, 2, 2)
C := blas.Zeros(2, 2)
result, _ = backend.Sgemm(blas.NoTrans, blas.NoTrans, 1.0, A, B, 0.0, C)
```

## GPU Backend Pattern

Each GPU backend embeds a `gpuBase` struct that provides all BLAS operations. The backend only needs to implement three methods:

- `upload(data []byte) (handle, error)` -- send data to the GPU
- `download(handle, size) ([]byte, error)` -- retrieve data from the GPU
- `free(handle) error` -- release GPU memory

The actual arithmetic is performed by the CPU reference (CpuBlas), while the GPU memory pipeline is fully exercised. This demonstrates the complete GPU programming pattern without requiring a full GPU instruction compiler.

## Testing

```bash
go test ./... -v -cover
```

## Dependencies

- Layer 4: `vendor-api-simulators` -- provides CUDA, Metal, OpenCL, Vulkan, WebGPU, and OpenGL runtime simulators
- Layer 5: `compute-runtime` -- provides the underlying device management and memory operations
