# gpu-runtime

**G09 Layer 5** — Abstract GPU compute runtime.  Selects the best available
GPU backend at process startup and provides a unified dispatch API.

## Backend priority

```
1. Metal   — macOS / Apple Silicon / Intel Mac
2. CUDA    — Linux / Windows with NVIDIA GPU + driver
3. CPU     — pure-Rust fallback, always available
```

`Runtime::global()` returns the singleton (initialised once), or use
`Runtime::detect()` for a fresh instance.  `Runtime::cpu_only()` forces the
CPU backend — useful in tests and headless environments.

## Quick start

```rust
use gpu_runtime::{Runtime, Shaders};

fn cpu_invert(src: &[u8], dst: &mut [u8], _uni: &[u8]) {
    for i in (0..src.len()).step_by(4) {
        dst[i]   = 255 - src[i];
        dst[i+1] = 255 - src[i+1];
        dst[i+2] = 255 - src[i+2];
        dst[i+3] = src[i+3];
    }
}

static INVERT_SHADERS: Shaders = Shaders {
    metal: Some(include_str!("shaders/invert.metal")),
    cuda:  Some(include_str!("shaders/invert.cu")),
    cpu:   Some(cpu_invert),
};

pub fn invert(pixels: &[u8], pixel_count: usize) -> Vec<u8> {
    Runtime::global()
        .run_pixels(&INVERT_SHADERS, "gpu_invert", pixels, &[], pixel_count)
        .expect("invert failed")
}
```

## Dispatch model

### `run_1d` — one thread per output byte

```
src: &[u8]   uniforms: &[u8]   count: usize (output bytes)
→  Vec<u8>  (count bytes)
```

Metal: `enc.dispatch_threads_1d(count as u32, tpg)`  
CUDA:  `grid = ceil(count / 256)` blocks of 256 threads  
CPU:   `f(src, dst, uniforms)` where `dst.len() == count`

### `run_pixels` — one thread per RGBA pixel

```
src: &[u8]   uniforms: &[u8]   pixel_count: usize
→  Vec<u8>  (pixel_count * 4 bytes)
```

Shader convention: `gid` = pixel index, byte offset = `gid * 4`.
Use this for image operations that read/write all 4 channels per thread.

## Shader bundles

```rust
pub struct Shaders {
    pub metal: Option<&'static str>,   // MSL compute kernel source
    pub cuda:  Option<&'static str>,   // CUDA C kernel source  
    pub cpu:   Option<fn(&[u8], &mut [u8], &[u8])>,  // Rust fallback
}
```

If the active backend has no shader for an operation, `GpuError::NoShaderForBackend` is returned.

## Features

| Feature | Default | Effect |
|---------|---------|--------|
| `metal` | ✓ | Link Metal.framework; enable Metal backend (macOS only) |

Disable with `--no-default-features` for CPU+CUDA-only builds that do not load
Metal.framework at startup (useful in sandboxed test environments).

## Thread safety

`Runtime` is `Send + Sync`.  A `Mutex<RuntimeInner>` serialises GPU dispatch —
only one thread submits work at a time.  For concurrent dispatch, create
multiple `Runtime` instances.

## How it fits in the stack

```
metal-compute  cuda-compute   (Layer 4B: backend implementations)
      ↓              ↓
         gpu-runtime           ← this crate (Layer 5: unified API)
              ↓
         image-gpu-core        (Layer 6: image operations)
```
