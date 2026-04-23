# metal-compute

**G09 Layer 4B** — Real Metal GPU compute on macOS via `objc-bridge`.

This crate wraps Apple's [Metal](https://developer.apple.com/metal/) compute
API into clean Rust types — `MetalDevice`, `MetalBuffer`, `MetalLibrary`,
`MetalComputePipeline`, and `MetalCommandQueue`.  No third-party crate
dependencies; Metal is called directly through ObjC message dispatch provided
by `objc-bridge`.

## Platform

macOS / Apple Silicon / Intel Mac only.  On other platforms every constructor
returns `Err(MetalError::NotSupported)`.

## Usage

```rust
use metal_compute::{MetalDevice, MetalError};

const MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

kernel void gpu_invert(
    device const uchar* src [[buffer(0)]],
    device       uchar* dst [[buffer(1)]],
    device const uchar* uni [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    uint o = gid * 4u;
    dst[o+0] = 255u - src[o+0];
    dst[o+1] = 255u - src[o+1];
    dst[o+2] = 255u - src[o+2];
    dst[o+3] = src[o+3];
}
"#;

fn invert_pixels(pixels: &[u8], pixel_count: usize) -> Result<Vec<u8>, MetalError> {
    let device = MetalDevice::new()?;
    let queue  = device.command_queue();

    let src_buf = device.alloc_with_bytes(pixels);
    let dst_buf = device.alloc(pixel_count * 4);

    let lib      = device.compile(MSL)?;
    let func     = lib.function("gpu_invert")?;
    let pipeline = device.pipeline(&func)?;
    let tpg      = pipeline.preferred_threads_1d();

    queue.dispatch(|enc| {
        enc.set_pipeline(&pipeline);
        enc.set_buffer(&src_buf, 0);
        enc.set_buffer(&dst_buf, 1);
        enc.set_buffer(&device.alloc(4), 2); // unused uniforms
        enc.dispatch_threads_1d(pixel_count as u32, tpg);
    });

    Ok(dst_buf.to_vec())
}
```

## Key types

| Type | Description |
|------|-------------|
| `MetalDevice` | Wraps `id<MTLDevice>` — creates buffers, compiles libraries, creates pipelines |
| `MetalBuffer` | GPU/CPU shared memory (`MTLResourceStorageModeShared`) |
| `MetalLibrary` | Compiled MSL library (`id<MTLLibrary>`) |
| `MetalFunction` | Reference to a compute kernel function |
| `MetalComputePipeline` | Compiled PSO ready for dispatch |
| `MetalCommandQueue` | Serialised command queue — use `dispatch(|enc| { ... })` |

## Unified memory on Apple Silicon

On M-series chips (M1, M2, M3, …) the CPU and GPU share one physical memory
die.  `MetalBuffer` allocated with `Shared` storage mode is a direct pointer
into this unified pool — both sides see writes immediately with zero copy cost.

## Thread safety

`MetalDevice`, `MetalLibrary`, `MetalComputePipeline`, and `MetalCommandQueue`
are `Send + Sync` (Metal objects are documented thread-safe).
`MetalBuffer` is `Send` but not `Sync` — concurrent mutable access is the
caller's responsibility.

## How it fits in the stack

```
metal-compute   ← this crate (Metal compute primitives)
      ↑
gpu-runtime     (abstract runtime: Metal → CUDA → CPU)
      ↑
image-gpu-core  (GPU-accelerated image operations)
```

## Running tests

Tests require a real Metal GPU (macOS machine):

```sh
cargo test -p metal-compute -- --nocapture
```
