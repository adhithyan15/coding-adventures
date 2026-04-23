# image-gpu-core

**IMG06** — GPU-accelerated point operations on `PixelContainer`.

Every function transforms each pixel independently — embarrassingly parallel
workloads that map perfectly to GPU compute shaders.  The API mirrors
`image-point-ops` (IMG03) but routes work through `gpu-runtime`, which
selects Metal (macOS), CUDA (NVIDIA), or pure-Rust CPU as the backend.

## Operations

| Function | Description |
|----------|-------------|
| `gpu_invert` | Invert RGB channels; alpha unchanged |
| `gpu_colour_matrix` | Apply 3×3 matrix in linear light; alpha unchanged |
| `gpu_greyscale` | Weighted luminance to grey (Rec.709, BT.601, or average) |
| `gpu_gamma` | Power-law gamma in linear light |
| `gpu_brightness` | Additive brightness shift in sRGB u8, clamped |

## Quick start

```rust
use pixel_container::PixelContainer;
use image_gpu_core::{gpu_invert, gpu_colour_matrix, LuminanceWeights};

// Load or construct an RGBA8 image
let mut img = PixelContainer::new(1920, 1080);
img.fill(128, 64, 32, 255);

// Invert — transparent backend selection (Metal on Mac, CUDA on NVIDIA, CPU otherwise)
let inverted = gpu_invert(&img).unwrap();

// Greyscale with Rec.709 weights
let grey = image_gpu_core::gpu_greyscale(&img, LuminanceWeights::Rec709).unwrap();

// Gamma darkening (γ = 2.2)
let darkened = image_gpu_core::gpu_gamma(&img, 2.2).unwrap();

// Colour matrix: swap R↔B
let swap = [[0.0_f32, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
let swapped = gpu_colour_matrix(&img, &swap).unwrap();
```

## Shader sources

Each operation ships three shader variants compiled into the binary:

| Backend | Language | Files |
|---------|----------|-------|
| Metal   | MSL      | `shaders/metal/*.metal` |
| CUDA    | CUDA C   | `shaders/cuda/*.cu` |
| CPU     | Rust fn  | `src/lib.rs` (inline) |

The GPU driver compiles MSL (via the Metal driver) or CUDA C (via NVRTC) to
native GPU binary at runtime.  The CPU fallback runs the identical logic in
Rust — useful for correctness testing and non-GPU environments.

## Colorspace

`PixelContainer` stores RGBA8 in sRGB encoding.  Operations requiring accurate
arithmetic (colour matrix, gamma, greyscale) decode sRGB → linear light,
operate, then re-encode.  The sRGB transfer function is implemented identically
in Rust, MSL, and CUDA C to within ±1 LSB.

## Thread dispatch model

Each GPU thread handles one RGBA pixel (4 bytes):

```
gid = thread_position_in_grid   (Metal)
    = blockIdx.x * 256 + threadIdx.x  (CUDA)
byte offset = gid * 4  →  [R, G, B, A]
```

## Features

| Feature | Default | Effect |
|---------|---------|--------|
| `metal` | ✓ | Enable Metal backend via `gpu-runtime/metal` |

## Running tests

Tests use `Runtime::cpu_only()` — no GPU required:

```sh
cargo test -p image-gpu-core -- --nocapture
```

To test the Metal GPU path (macOS only):

```sh
cargo test -p image-gpu-core -- --nocapture --ignored
```

## How it fits in the stack

```
pixel-container                (RGBA8 image buffer)
gpu-runtime                    (Metal → CUDA → CPU dispatch)
     ↓
image-gpu-core   ← this crate (GPU point operations on PixelContainer)
     ↓
image-codecs / paint-vm        (encode, render, display)
```
