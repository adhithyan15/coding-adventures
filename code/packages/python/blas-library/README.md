# BLAS Library

A pluggable BLAS (Basic Linear Algebra Subprograms) library with 7 swappable backends. Part of the coding-adventures accelerator computing stack (Layer 3).

## What is BLAS?

BLAS defines standard linear algebra operations at three levels:

- **Level 1** (1979): Vector-Vector -- SAXPY, DOT, NRM2, SCAL, ASUM, IAMAX
- **Level 2** (1988): Matrix-Vector -- GEMV, GER
- **Level 3** (1990): Matrix-Matrix -- GEMM, SYMM, Batched GEMM

Plus ML extensions: ReLU, softmax, layer normalization, attention, conv2d.

## Quick Start

```python
from blas_library import create_blas, Matrix, Vector, Transpose

# Auto-select the best available backend
blas = create_blas("auto")

# Or pick a specific one
blas = create_blas("cpu")    # Pure Python reference
blas = create_blas("cuda")   # NVIDIA GPU
blas = create_blas("metal")  # Apple Silicon

# Matrix multiply: C = A * B
A = Matrix(data=[1, 2, 3, 4, 5, 6], rows=2, cols=3)
B = Matrix(data=[7, 8, 9, 10, 11, 12], rows=3, cols=2)
C = Matrix(data=[0, 0, 0, 0], rows=2, cols=2)

result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, A, B, 0.0, C)
# result.data == [58.0, 64.0, 139.0, 154.0]
```

## The Seven Backends

| Backend | Wraps | Target Hardware |
|---------|-------|-----------------|
| CpuBlas | Pure Python | Any CPU |
| CudaBlas | CUDARuntime | NVIDIA GPUs |
| OpenClBlas | CLContext | Any OpenCL device |
| MetalBlas | MTLDevice | Apple Silicon |
| VulkanBlas | VkDevice | Any Vulkan device |
| WebGpuBlas | GPUDevice | Browsers + native |
| OpenGlBlas | GLContext | OpenGL 4.3+ |

All backends produce identical results (within floating-point tolerance).

## Layer Position

```
Layer 4: Vendor API Simulators (CUDA, OpenCL, Metal, Vulkan, WebGPU, OpenGL)
    |
Layer 3: BLAS Library  <-- THIS PACKAGE
    |
Layer 2: Tensor + Autograd (future)
```

## Dependencies

- `coding-adventures-vendor-api-simulators` (for GPU backends)
- No dependencies for the CPU backend (pure Python)
