# cuda-compute

**G09 Layer 4B** — Real CUDA GPU compute via dynamically-loaded `libcuda` and NVRTC.

This crate loads the CUDA Driver API (`libcuda.so.1`) and the NVIDIA Runtime
Compilation library (`libnvrtc.so`) at runtime using `dlopen`/`LoadLibrary`.
If CUDA is not installed the constructors return
`Err(CudaError::NotAvailable)` — the binary does not fail to start.

Zero link-time NVIDIA dependency: the crate adds no `#[link]` attributes for
CUDA.

## Usage

```rust
use cuda_compute::{CudaDevice, CudaError};

const KERNEL: &str = r#"
extern "C" __global__ void gpu_invert(
    const unsigned char* src,
    unsigned char*       dst,
    unsigned int         pixel_count
) {
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= pixel_count) return;
    unsigned int o = gid * 4u;
    dst[o+0] = 255u - src[o+0];
    dst[o+1] = 255u - src[o+1];
    dst[o+2] = 255u - src[o+2];
    dst[o+3] = src[o+3];
}
"#;

fn invert_pixels(pixels: &[u8]) -> Result<Vec<u8>, CudaError> {
    let device = CudaDevice::new(0)?;   // Err(NotAvailable) if no CUDA
    let src    = device.alloc_with_bytes(pixels)?;
    let dst    = device.alloc(pixels.len())?;

    let module   = device.compile(KERNEL)?;
    let function = module.function("gpu_invert")?;

    let n          = (pixels.len() / 4) as u32;
    let block_size = 256u32;
    let grid_size  = (n + block_size - 1) / block_size;

    // argument list is architecture-specific; see cuda-compute docs
    device.launch(&function, [grid_size, 1, 1], [block_size, 1, 1], &mut [])?;
    device.synchronize()?;
    device.download(&dst)
}
```

## Dynamic loading

Libraries are probed in order:

| Library | Names tried |
|---------|-------------|
| CUDA Driver API | `libcuda.so.1`, `libcuda.so` (Linux) / `nvcuda.dll` (Windows) |
| NVRTC | `libnvrtc.so.12`, `libnvrtc.so` (Linux) / `nvrtc64_120_0.dll` (Windows) |

On macOS the crate always returns `Err(CudaError::NotAvailable)` — NVIDIA
does not ship a macOS driver.

## How it fits in the stack

```
cuda-compute    ← this crate (CUDA compute primitives)
     ↑
gpu-runtime     (abstract runtime: Metal → CUDA → CPU)
     ↑
image-gpu-core  (GPU-accelerated image operations)
```

## Running tests

```sh
cargo test -p cuda-compute -- --nocapture
```

Tests pass on non-CUDA machines — they verify that `CudaDevice::new()` returns
`Err(NotAvailable)` gracefully.
