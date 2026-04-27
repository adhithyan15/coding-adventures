# IMG06 — GPU Acceleration Bridge

## Overview

The CPU implementations in IMG01–IMG05 are correct and portable. They are also
slow for large images: a Gaussian blur on a 4K frame at 24 fps requires ~2
billion multiply-accumulates per second. A single CPU core delivers roughly
100–200 million f32 FLOPS in a scalar loop; with SIMD, perhaps 4–8× more.
The arithmetic cannot fit in real time on CPU alone.

A modern GPU turns this budget around:

```
CPU (12 cores, AVX2):      ~50–100 GFLOPS f32
Entry-level discrete GPU:  ~2000 GFLOPS f32  (40× faster)
Mid-range GPU (2024):      ~15000 GFLOPS f32 (300× faster)
```

The reason is architecture. A GPU contains thousands of small shader cores that
execute the **same program on different data** simultaneously (SIMT — Single
Instruction, Multiple Threads). Image processing is a near-perfect fit: the
same kernel is applied independently to every pixel (or small tile of pixels).

IMG06 defines two GPU back-ends that share a common WGSL shader library:

```
Back-end A:  Rust crate  image-gpu-core
             API: Rust   →  wgpu  →  Metal (macOS) / Vulkan (Linux) / DX12 (Windows)
             FFI: C-ABI exported from the crate → callable from Python, Ruby, Go, …

Back-end B:  TypeScript  image-gpu-ts
             API: TypeScript → navigator.gpu (WebGPU) → native driver in browser
```

Both back-ends run identical WGSL compute shaders and produce bit-identical
output (within f32 rounding) to the CPU reference implementations in IMG01–IMG05.

---

## 1. Why Two Back-Ends?

### Rust + wgpu (native)

**wgpu** is a Rust implementation of the WebGPU API that targets real native
GPU APIs:

```
wgpu on macOS    → Metal
wgpu on Linux    → Vulkan (preferred) or OpenGL 4.3
wgpu on Windows  → Direct3D 12 (preferred) or Vulkan
wgpu anywhere    → software (wgpu's own CPU WGSL interpreter, for CI)
```

The key advantage: one codebase, one shader language (WGSL), every desktop
platform. No conditional compilation for Metal vs Vulkan vs DX12. The wgpu
crate handles backend selection at runtime.

A Rust crate can expose a **C-ABI** (`extern "C"` functions, `#[no_mangle]`)
that any language with a C FFI can call. Every language in this repo already has
a pattern for calling C libraries:

```
Python:    ctypes / cffi
Ruby:      Fiddle / ffi gem
Go:        cgo
TypeScript (Node): node-ffi-napi or N-API native addon
Java/Kotlin:   JNI or JNA
Rust itself:   direct crate dependency
```

### TypeScript + WebGPU (browser)

In the browser there is no Rust, no wgpu, no native driver access. The browser
exposes the **WebGPU API** (`navigator.gpu`), which speaks the same conceptual
model as wgpu. The same WGSL shaders run unchanged. The TypeScript package calls
`navigator.gpu` directly, with no Rust in the loop.

This means a web application can apply GPU-accelerated convolutions and LUTs
without shipping a WASM binary or a native module.

---

## 2. WGSL Shader Library

WGSL (WebGPU Shading Language) is the single shader language supported by both
wgpu and the browser WebGPU implementation. It is a statically-typed, memory-
safe language with no undefined behaviour. All shaders in this series are
written in WGSL and stored in the shared directory `shaders/`.

### 2.1 Compute shader model

Each GPU operation is implemented as a **compute shader** dispatched over a
2D workgroup grid:

```
Workgroup size: 8×8 threads (64 threads per workgroup)

Dispatch for a W×H image:
  groups_x = ceil(W / 8)
  groups_y = ceil(H / 8)
  dispatch(groups_x, groups_y, 1)
```

Each thread handles one output pixel at coordinates:

```wgsl
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if x >= uniforms.width || y >= uniforms.height { return; }
    // ... process pixel (x, y) ...
}
```

The early-return guards against threads that fall outside the image boundary
when the image dimensions are not multiples of 8.

### 2.2 Texture layout

Images are uploaded to the GPU as `texture_2d<f32>` in RGBA32F format:

```
Even for RGB images: upload with a dummy alpha channel (α = 1.0).
Even for grayscale:  upload R=G=B=value, α=1.0.
```

This uniformity simplifies shader code: every shader reads `vec4<f32>` and the
caller strips unused channels on readback.

Storage textures (`texture_storage_2d<rgba32float, write>`) are used for
output, because compute shaders cannot write to sampled textures.

### 2.3 Uniform buffer layout

Each shader receives a uniform buffer with at minimum:

```wgsl
struct Uniforms {
    width:  u32,
    height: u32,
    // operation-specific parameters follow
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
```

All uniform structs must be `16`-byte aligned (WebGPU requirement). Pad with
dummy fields if necessary.

---

## 3. Shader Catalogue

### 3.1 Point operations (`shaders/point_ops.wgsl`)

Generic shader parameterised by a 4×4 colour matrix M applied per pixel.
Covers brightness, contrast, colour balance, channel swap, and any linear
colour transform:

```wgsl
struct Uniforms {
    width:  u32,
    height: u32,
    matrix: mat4x4<f32>,   // applied to (R, G, B, 1) homogeneous column vector
    _pad:   vec2<f32>,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;  let y = gid.y;
    if x >= uniforms.width || y >= uniforms.height { return; }
    let coord = vec2<i32>(i32(x), i32(y));
    let colour = textureLoad(src, coord, 0);
    let out = uniforms.matrix * vec4<f32>(colour.rgb, 1.0);
    textureStore(dst, coord, vec4<f32>(clamp(out.xyz, vec3(0.0), vec3(1.0)), colour.a));
}
```

For non-linear operations (gamma, sRGB conversion), the host builds a 1D LUT
texture and the shader samples it (§3.4).

### 3.2 2D convolution (`shaders/conv2d.wgsl`)

Supports kernels up to 15×15 (radius ≤ 7). The kernel is uploaded as a uniform
array of 225 f32 values (15×15 maximum, padded with zeros for smaller kernels).

Padding mode is selected by a uniform integer:

```wgsl
const PAD_ZERO      : u32 = 0u;
const PAD_REPLICATE : u32 = 1u;
const PAD_REFLECT   : u32 = 2u;
const PAD_WRAP      : u32 = 3u;
```

The shader loads the neighbourhood for each pixel, multiplies by the kernel
weights, and accumulates in f32:

```wgsl
var acc = vec4<f32>(0.0);
for (var j: i32 = -radius; j <= radius; j++) {
    for (var i: i32 = -radius; i <= radius; i++) {
        let sample_coord = pad_coord(vec2<i32>(ix + i, iy + j), uniforms.padding_mode);
        let sample = textureLoad(src, sample_coord, 0);
        let w = uniforms.kernel[(j + radius) * (2 * radius + 1) + (i + radius)];
        acc += sample * w;
    }
}
textureStore(dst, vec2<i32>(ix, iy), acc);
```

### 3.3 Separable convolution (`shaders/conv_sep_h.wgsl`, `shaders/conv_sep_v.wgsl`)

Two-pass separable convolution for Gaussian blur and other separable filters.
Runs two dispatches: horizontal then vertical, with an intermediate texture.

The horizontal shader:

```wgsl
// Reads from src, writes to tmp (intermediate texture).
// Kernel: 1D array of 2*radius+1 weights, applied in X direction.
```

The vertical shader:

```wgsl
// Reads from tmp, writes to dst.
// Same kernel, applied in Y direction.
```

Using a shared intermediate texture avoids allocating a new buffer per frame.
The host is responsible for creating `tmp` at the same dimensions as `src`.

### 3.4 1D LUT application (`shaders/lut1d.wgsl`)

The LUT is uploaded as a 1D texture (`texture_1d<f32>`) of N entries per
channel. The shader samples it with linear interpolation:

```wgsl
@group(0) @binding(2) var lut_r: texture_1d<f32>;
@group(0) @binding(3) var lut_g: texture_1d<f32>;
@group(0) @binding(4) var lut_b: texture_1d<f32>;
@group(0) @binding(5) var lut_sampler: sampler;

fn apply_1d_luts(colour: vec3<f32>) -> vec3<f32> {
    let r = textureSample(lut_r, lut_sampler, colour.r).r;
    let g = textureSample(lut_g, lut_sampler, colour.g).g;
    let b = textureSample(lut_b, lut_sampler, colour.b).b;
    return vec3<f32>(r, g, b);
}
```

### 3.5 3D LUT application (`shaders/lut3d.wgsl`)

The 3D LUT lattice is uploaded as a `texture_3d<f32>` of size N×N×N:

```wgsl
@group(0) @binding(2) var lut3d:    texture_3d<f32>;
@group(0) @binding(3) var lut_samp: sampler;

fn apply_3d_lut(colour: vec3<f32>) -> vec3<f32> {
    let n  = f32(textureDimensions(lut3d).x);
    // Map [0,1] to [0.5/N, 1-0.5/N] to sample at texel centres.
    let uv = colour * ((n - 1.0) / n) + (0.5 / n);
    return textureSample(lut3d, lut_samp, uv).rgb;
}
```

Hardware trilinear interpolation is used (sampler `minFilter = linear`,
`magFilter = linear`). This matches the CPU trilinear implementation in IMG02.

---

## 4. The Rust Crate: `image-gpu-core`

### 4.1 Crate structure

```
code/packages/rust/image-gpu-core/
├── src/
│   ├── lib.rs          — public Rust API + C-ABI exports
│   ├── device.rs       — wgpu device/queue/adapter initialisation
│   ├── pipeline.rs     — shader compilation and pipeline cache
│   ├── texture.rs      — Image<P> ↔ wgpu texture upload/download helpers
│   ├── ops/
│   │   ├── conv.rs     — convolve2d, gaussian_blur, separable
│   │   ├── lut.rs      — apply_lut1d, apply_lut3d
│   │   └── point.rs    — colour matrix, gamma
│   └── ffi.rs          — extern "C" functions for cross-language binding
├── shaders/            — WGSL source files (compiled at build time via naga)
│   ├── conv2d.wgsl
│   ├── conv_sep_h.wgsl
│   ├── conv_sep_v.wgsl
│   ├── lut1d.wgsl
│   ├── lut3d.wgsl
│   └── point_ops.wgsl
├── tests/
│   ├── reference_images/ — small PNG test images
│   └── compare_cpu_gpu.rs — pixel-by-pixel diff against CPU reference
├── Cargo.toml
├── BUILD
├── README.md
└── CHANGELOG.md
```

### 4.2 Device initialisation

```rust
pub struct GpuContext {
    instance: wgpu::Instance,
    adapter:  wgpu::Adapter,
    device:   wgpu::Device,
    queue:    wgpu::Queue,
}

impl GpuContext {
    pub async fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::default();
        let adapter  = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or(GpuError::NoAdapter)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;
        Ok(GpuContext { instance, adapter, device, queue })
    }
}
```

A global `OnceCell<GpuContext>` is initialised on the first GPU call and
reused for the lifetime of the process. The heavy adapter/device creation
happens once; individual operations pay only the shader-dispatch cost.

### 4.3 Texture upload and download

```rust
// Upload Image<RGBA32F> to a wgpu texture.
fn upload_texture(ctx: &GpuContext, image: &Image<RGBA32F>) -> wgpu::Texture { … }

// Download a wgpu texture to Image<RGBA32F> (blocking — waits for GPU).
fn download_texture(ctx: &GpuContext, texture: &wgpu::Texture, w: u32, h: u32) -> Image<RGBA32F> { … }
```

Both functions exist because the GPU/CPU data transfer is the dominant latency
in short pipelines. Callers that chain multiple GPU operations should keep the
image on the GPU (pass the texture between operations) and only download at the
end.

### 4.4 Rust public API

```rust
// All functions are synchronous from the caller's perspective.
// Internally they submit a command buffer and wait for completion.

pub fn gpu_convolve2d(
    ctx:     &GpuContext,
    src:     &Image<RGBA32F>,
    kernel:  &[f32],        // (2r+1)^2 entries, row-major
    radius:  u32,
    padding: PaddingMode,
) -> Result<Image<RGBA32F>, GpuError>

pub fn gpu_gaussian_blur(
    ctx:     &GpuContext,
    src:     &Image<RGBA32F>,
    sigma:   f32,
    padding: PaddingMode,
) -> Result<Image<RGBA32F>, GpuError>

pub fn gpu_apply_lut3d(
    ctx:    &GpuContext,
    src:    &Image<RGBA32F>,
    lut:    &Lut3d,
) -> Result<Image<RGBA32F>, GpuError>

pub fn gpu_apply_lut1d(
    ctx:     &GpuContext,
    src:     &Image<RGBA32F>,
    r_lut:   &Lut1dF32,
    g_lut:   &Lut1dF32,
    b_lut:   &Lut1dF32,
) -> Result<Image<RGBA32F>, GpuError>

pub fn gpu_colour_matrix(
    ctx:    &GpuContext,
    src:    &Image<RGBA32F>,
    matrix: &[[f32; 4]; 4],
) -> Result<Image<RGBA32F>, GpuError>
```

---

## 5. The C-ABI FFI Layer (`ffi.rs`)

Every function in the public Rust API has a corresponding `extern "C"` wrapper.
The wrappers use only C-compatible types: raw pointers, primitive integers,
C-style structs.

### 5.1 Opaque handle pattern

The GPU context and image buffers are returned as opaque pointers:

```rust
// Opaque handle to GpuContext (heap-allocated, caller must free).
pub type ImgGpuCtx = *mut GpuContext;

// Opaque handle to Image<RGBA32F> (heap-allocated, caller must free).
pub type ImgBuffer = *mut Image<RGBA32F>;

#[no_mangle]
pub extern "C" fn img_gpu_ctx_new() -> ImgGpuCtx {
    match pollster::block_on(GpuContext::new()) {
        Ok(ctx) => Box::into_raw(Box::new(ctx)),
        Err(_)  => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn img_gpu_ctx_free(ctx: ImgGpuCtx) {
    if !ctx.is_null() { unsafe { drop(Box::from_raw(ctx)) }; }
}

#[no_mangle]
pub extern "C" fn img_buffer_free(buf: ImgBuffer) {
    if !buf.is_null() { unsafe { drop(Box::from_raw(buf)) }; }
}
```

### 5.2 Image data in/out

Images cross the FFI boundary as raw byte arrays with explicit dimensions:

```rust
// Create an ImgBuffer from a raw RGBA32F byte slice.
// data: pointer to width*height*4 f32 values, row-major, interleaved RGBA.
#[no_mangle]
pub extern "C" fn img_buffer_from_bytes(
    data:   *const f32,
    width:  u32,
    height: u32,
) -> ImgBuffer { … }

// Copy ImgBuffer pixels into a caller-provided f32 array.
// out must be pre-allocated to width*height*4 f32 values.
#[no_mangle]
pub extern "C" fn img_buffer_to_bytes(buf: ImgBuffer, out: *mut f32) { … }

#[no_mangle]
pub extern "C" fn img_buffer_width(buf: ImgBuffer) -> u32 { … }

#[no_mangle]
pub extern "C" fn img_buffer_height(buf: ImgBuffer) -> u32 { … }
```

### 5.3 Operation wrappers

```rust
// Gaussian blur. Returns a new ImgBuffer (caller must free).
#[no_mangle]
pub extern "C" fn img_gpu_gaussian_blur(
    ctx:          ImgGpuCtx,
    src:          ImgBuffer,
    sigma:        f32,
    padding_mode: u32,   // 0=zero, 1=replicate, 2=reflect, 3=wrap
) -> ImgBuffer { … }

// Apply a 3D LUT.
// lattice: N*N*N*3 f32 values, B-major ordering (matching .cube format).
#[no_mangle]
pub extern "C" fn img_gpu_apply_lut3d(
    ctx:     ImgGpuCtx,
    src:     ImgBuffer,
    lattice: *const f32,
    n:       u32,        // lattice dimension
) -> ImgBuffer { … }
```

The full set of wrappers mirrors the Rust public API one-to-one. Every function
that allocates returns a pointer that must be freed with `img_buffer_free`.

### 5.4 Header file (`image_gpu.h`)

The crate generates a C header file (via `cbindgen`) at build time:

```
code/packages/rust/image-gpu-core/image_gpu.h
```

This header is the single source of truth for the C-ABI contract. Language
bindings in Python, Ruby, Go, etc. load the shared library and declare
functions by referencing this header.

---

## 6. Per-Language Bindings

Each language's binding package is a thin wrapper around the C-ABI. It handles:

1. Loading the shared library (`libimage_gpu_core.so` / `.dylib` / `.dll`)
2. Declaring the C functions
3. Converting native image types (NumPy array, Ruby Fiddle, etc.) to/from the
   raw byte representation expected by the FFI
4. Calling `img_gpu_ctx_new()` lazily and caching the context
5. Ensuring handles are freed when the native wrapper object is garbage-collected

### Python (ctypes)

```python
import ctypes, numpy as np
_lib = ctypes.CDLL("libimage_gpu_core.so")

_lib.img_gpu_ctx_new.restype  = ctypes.c_void_p
_lib.img_gpu_gaussian_blur.argtypes = [
    ctypes.c_void_p,  # ctx
    ctypes.c_void_p,  # src
    ctypes.c_float,   # sigma
    ctypes.c_uint32,  # padding_mode
]
_lib.img_gpu_gaussian_blur.restype = ctypes.c_void_p

def gaussian_blur(image: np.ndarray, sigma: float, padding="replicate") -> np.ndarray:
    """image: H×W×4 float32 array (RGBA, linear light)"""
    h, w, _ = image.shape
    src = _lib.img_buffer_from_bytes(image.ctypes.data_as(ctypes.POINTER(ctypes.c_float)), w, h)
    dst = _lib.img_gpu_gaussian_blur(_ctx(), src, sigma, _padding_int(padding))
    out = np.empty((h, w, 4), dtype=np.float32)
    _lib.img_buffer_to_bytes(dst, out.ctypes.data_as(ctypes.POINTER(ctypes.c_float)))
    _lib.img_buffer_free(dst)
    _lib.img_buffer_free(src)
    return out
```

### Go (cgo)

```go
// #cgo LDFLAGS: -L${SRCDIR} -limage_gpu_core
// #include "image_gpu.h"
import "C"
import "unsafe"

func GaussianBlur(pixels []float32, w, h int, sigma float32) []float32 {
    ctx := C.img_gpu_ctx_new()
    src := C.img_buffer_from_bytes((*C.float)(unsafe.Pointer(&pixels[0])), C.uint32_t(w), C.uint32_t(h))
    dst := C.img_gpu_gaussian_blur(ctx, src, C.float(sigma), 1 /*replicate*/)
    out := make([]float32, w*h*4)
    C.img_buffer_to_bytes(dst, (*C.float)(unsafe.Pointer(&out[0])))
    C.img_buffer_free(dst)
    C.img_buffer_free(src)
    return out
}
```

---

## 7. TypeScript + WebGPU (`image-gpu-ts`)

The TypeScript package provides the same operations as the Rust crate, calling
`navigator.gpu` directly. It shares the WGSL shader sources with the Rust crate
(copied into the package's `shaders/` directory at build time).

### 7.1 Package structure

```
code/packages/typescript/image-gpu-ts/
├── src/
│   ├── index.ts        — public API (re-exports)
│   ├── context.ts      — GPUDevice acquisition and caching
│   ├── texture.ts      — ImageData / Float32Array ↔ GPUTexture helpers
│   ├── pipeline.ts     — shader module compilation and pipeline cache
│   ├── ops/
│   │   ├── conv.ts     — convolve2d, gaussianBlur
│   │   ├── lut.ts      — applyLut1d, applyLut3d
│   │   └── point.ts    — colourMatrix
├── shaders/            — WGSL files (same as Rust crate's shaders/)
├── tests/
└── package.json
```

### 7.2 Device acquisition

```typescript
let _device: GPUDevice | null = null;

async function getDevice(): Promise<GPUDevice> {
    if (_device) return _device;
    if (!navigator.gpu) throw new Error("WebGPU not supported");
    const adapter = await navigator.gpu.requestAdapter({ powerPreference: "high-performance" });
    if (!adapter) throw new Error("No GPU adapter found");
    _device = await adapter.requestDevice();
    return _device;
}
```

### 7.3 Image ↔ GPUTexture

Images enter the TypeScript API as `Float32Array` with layout identical to the
Rust FFI (width × height × 4 f32 values, RGBA interleaved, row-major):

```typescript
function uploadTexture(device: GPUDevice, data: Float32Array, w: number, h: number): GPUTexture {
    const texture = device.createTexture({
        size: [w, h],
        format: "rgba32float",
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    });
    device.queue.writeTexture(
        { texture },
        data,
        { bytesPerRow: w * 16 },   // 4 channels × 4 bytes
        [w, h],
    );
    return texture;
}

async function downloadTexture(device: GPUDevice, texture: GPUTexture, w: number, h: number): Promise<Float32Array> {
    const bufferSize = w * h * 16;
    const readBuf = device.createBuffer({ size: bufferSize, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
    const encoder = device.createCommandEncoder();
    encoder.copyTextureToBuffer({ texture }, { buffer: readBuf, bytesPerRow: w * 16 }, [w, h]);
    device.queue.submit([encoder.finish()]);
    await readBuf.mapAsync(GPUMapMode.READ);
    const result = new Float32Array(readBuf.getMappedRange().slice(0));
    readBuf.unmap();
    readBuf.destroy();
    return result;
}
```

### 7.4 Gaussian blur example

```typescript
export async function gaussianBlur(
    src:     Float32Array,
    width:   number,
    height:  number,
    sigma:   number,
    padding: PaddingMode = "replicate",
): Promise<Float32Array> {
    const device  = await getDevice();
    const srcTex  = uploadTexture(device, src, width, height);
    const dstTex  = device.createTexture({
        size: [width, height],
        format: "rgba32float",
        usage: GPUTextureUsage.STORAGE_BINDING | GPUTextureUsage.COPY_SRC,
    });
    const tmpTex  = device.createTexture({ /* same as dstTex */ });

    const { hKernel, vKernel } = buildGaussianKernels(sigma);
    // Pass 1: horizontal
    await runSeparablePass(device, srcTex, tmpTex, hKernel, "horizontal", padding, width, height);
    // Pass 2: vertical
    await runSeparablePass(device, tmpTex, dstTex, vKernel, "vertical",   padding, width, height);

    const result = await downloadTexture(device, dstTex, width, height);
    srcTex.destroy();  dstTex.destroy();  tmpTex.destroy();
    return result;
}
```

---

## 8. Testing Strategy

### 8.1 CPU–GPU bit parity

Every GPU operation is tested against the CPU reference implementation:

```
for each test image (small, medium, large, edge cases):
    cpu_out = cpu_gaussian_blur(image, sigma=2.0, padding=replicate)
    gpu_out = gpu_gaussian_blur(image, sigma=2.0, padding=replicate)
    max_diff = max over all pixels of |cpu_out[p] - gpu_out[p]|
    assert max_diff < 1e-4   // within f32 rounding
```

The tolerance of 1e-4 accommodates the hardware's fused multiply-add
(FMA) units on the GPU, which can accumulate sums in a different order than
the CPU loop. Bit-exact agreement is not possible across platforms; f32
rounding tolerance is.

### 8.2 WebGPU fallback in CI

CI runs on Linux without a discrete GPU. The wgpu CI configuration uses:

```
WGPU_BACKEND=vulkan   (if Vulkan software renderer is available)
WGPU_BACKEND=gl       (Mesa OpenGL software renderer on Ubuntu)
```

The wgpu software backend is slow (~100× slower than hardware) but functionally
correct. All parity tests pass in CI.

For the TypeScript WebGPU tests, CI uses a headless Chrome with the
`--enable-unsafe-webgpu` flag and the Swiftshader software rasteriser.

### 8.3 Cross-language FFI tests

A separate test suite calls the C-ABI from a small C test binary, Python, and
Go. These tests verify:

1. The shared library loads and the symbols resolve.
2. `img_gpu_ctx_new()` returns a non-null pointer.
3. A round-trip `img_buffer_from_bytes` → `img_buffer_to_bytes` preserves pixel
   values exactly.
4. `img_gpu_gaussian_blur` on a known test image matches the pre-computed
   CPU reference output (stored as a `.fbin` binary float file in `tests/`).

---

## 9. Performance Notes

### Pipelining

The dominant cost for a single GPU operation on a small image is not the shader
execution — it is the CPU/GPU data transfer. Upload + download for a 1920×1080
RGBA32F image transfers 8 MB; at PCIe 3.0 ×16 bandwidth (~12 GB/s), that is
~0.7 ms. The shader itself runs in ~0.05 ms.

Callers that need to apply multiple operations to the same image should keep the
data on the GPU between operations:

```rust
// Efficient: single upload, multiple GPU operations, single download.
let tex = upload_texture(&ctx, &src);
let tex = run_gaussian_blur(&ctx, &tex, sigma, padding);
let tex = run_apply_lut3d(&ctx, &tex, &lut);
let result = download_texture(&ctx, &tex);
```

### Workgroup size tuning

The 8×8 workgroup size (64 threads) is a conservative default that works on
all GPUs. High-end GPUs (e.g., Apple M-series with 32-wide SIMT) can benefit
from 16×16 (256 threads per workgroup). This is configurable at pipeline
compilation time; the default workgroup size is queried from
`adapter.limits().max_compute_invocations_per_workgroup` and rounded down to
the nearest power-of-two-square.

### Separable vs direct convolution on GPU

Unlike the CPU case where separability gives a large speedup, on the GPU the
crossover is different. GPU memory bandwidth is the bottleneck for large
kernels, and the intermediate texture in the separable pass doubles the memory
traffic. For radius ≤ 4 (9×9 or smaller), direct 2D convolution is often
faster on GPU; for radius ≥ 5, separable wins.

The API exposes both; `gpu_gaussian_blur` chooses automatically based on σ.

---

## 10. Interface Summary

```
image-gpu-core (Rust crate):
  GpuContext::new() -> GpuContext
  gpu_convolve2d(ctx, src, kernel, radius, padding) -> Image<RGBA32F>
  gpu_convolve_separable(ctx, src, h, v, radius, padding) -> Image<RGBA32F>
  gpu_gaussian_blur(ctx, src, sigma, padding) -> Image<RGBA32F>
  gpu_sobel(ctx, src) -> (Image<RGBA32F>, Image<RGBA32F>)
  gpu_apply_lut1d(ctx, src, r_lut, g_lut, b_lut) -> Image<RGBA32F>
  gpu_apply_lut3d(ctx, src, lut) -> Image<RGBA32F>
  gpu_colour_matrix(ctx, src, matrix) -> Image<RGBA32F>

C-ABI exports (same operations, prefixed img_gpu_):
  img_gpu_ctx_new() -> ImgGpuCtx
  img_gpu_ctx_free(ctx)
  img_buffer_from_bytes(data, w, h) -> ImgBuffer
  img_buffer_to_bytes(buf, out)
  img_buffer_free(buf)
  img_gpu_gaussian_blur(ctx, src, sigma, padding_mode) -> ImgBuffer
  img_gpu_apply_lut3d(ctx, src, lattice, n) -> ImgBuffer
  img_gpu_colour_matrix(ctx, src, matrix_16f) -> ImgBuffer

image-gpu-ts (TypeScript, browser WebGPU):
  gaussianBlur(src, w, h, sigma, padding) -> Promise<Float32Array>
  convolve2d(src, w, h, kernel, radius, padding) -> Promise<Float32Array>
  applyLut3d(src, w, h, lut3d) -> Promise<Float32Array>
  applyLut1d(src, w, h, rLut, gLut, bLut) -> Promise<Float32Array>
  colourMatrix(src, w, h, matrix) -> Promise<Float32Array>
```
