# G09 — Real GPU Hardware Backends

## Overview

The GPU simulation stack (G03–G06) is an educational model of how GPU hardware
works internally: compute units, warp scheduling, bank conflicts, memory
hierarchies.  It is correct and instructive, but it runs on the CPU.

This spec defines the **real GPU backend packages** — packages that issue
actual work to real GPU hardware on each platform, using the vendor APIs
directly.  These packages have no third-party dependencies; they call vendor
C APIs through zero-dependency `extern "C"` declarations, exactly as
`objc-bridge` calls Metal and CoreText.

---

## 1. Backend Landscape

Different platforms have different GPU compute APIs:

```
macOS / iOS / tvOS:
    Metal Compute — Apple's first-party GPU API
    C functions: MTLCreateSystemDefaultDevice (C)
    ObjC methods: newCommandQueue, newLibraryWithSource, dispatchThreadgroups, …
    Shader language: MSL (Metal Shading Language), C++-like

Linux / Windows (NVIDIA):
    CUDA — NVIDIA's proprietary GPU compute platform
    C functions: cuInit, cuDeviceGet, cuMemAlloc, cuLaunchKernel, …
    Shader language: CUDA C (compiled with nvcc to PTX / cubin)

Linux / Windows (AMD, Intel, NVIDIA):
    Vulkan Compute — cross-platform Khronos standard
    C functions: vkCreateInstance, vkCreateDevice, vkCreateComputePipeline, …
    Shader language: GLSL, compiled offline to SPIR-V

Linux / Windows (AMD):
    ROCm/HIP — AMD's CUDA-compatible platform
    C functions: hipInit, hipMalloc, hipLaunchKernel, …
    Shader language: HIP C (same as CUDA C; nvcc/hipcc dual-compilation)

Any platform (fallback):
    CPU — pure Rust, single-threaded or Rayon-parallel
    No shader language — plain Rust loops
```

Each backend is a **separate Rust crate** so that:

- Packages that only need CPU fallback do not link Metal, CUDA, or Vulkan.
- Platform-specific code is isolated; `#[cfg(target_vendor = "apple")]` blocks
  only appear inside `metal-compute`.
- CI can test `cpu-compute` everywhere; Metal/CUDA tests run only on the
  appropriate hardware.

---

## 2. The Abstraction Layer: `gpu-runtime`

All four real backends implement a single trait so that higher-level packages
(like `image-gpu-core`) do not need to import specific backends:

```rust
pub trait GpuBackend: Send + Sync {
    /// Allocate a buffer on the GPU (or unified memory).
    fn alloc(&self, len: usize) -> Box<dyn GpuBuffer>;

    /// Upload data from a CPU slice into an existing GPU buffer.
    fn upload(&self, buf: &dyn GpuBuffer, data: &[u8]);

    /// Download data from a GPU buffer into a CPU slice.
    fn download(&self, buf: &dyn GpuBuffer, out: &mut [u8]);

    /// Compile a shader from source code and function name.
    fn compile(&self, source: &str, entry_point: &str) -> Box<dyn GpuShader>;

    /// Dispatch a compute kernel.
    fn dispatch(
        &self,
        shader:  &dyn GpuShader,
        buffers: &[&dyn GpuBuffer],
        grid:    [u32; 3],   // number of threadgroups (x, y, z)
        block:   [u32; 3],   // threads per threadgroup (x, y, z)
    );
}

pub trait GpuBuffer: Send + Sync {
    fn len(&self) -> usize;
}

pub trait GpuShader: Send + Sync {}
```

`gpu-runtime` selects the best available backend at startup:

```
Priority (descending):
  1. Metal      — available on macOS / iOS
  2. CUDA       — available if libcuda.so / nvcuda.dll loads at runtime
  3. Vulkan     — available if libvulkan.so / vulkan-1.dll loads at runtime
  4. CPU        — always available
```

Selection is done with **runtime dynamic loading** (`libloading` crate, or hand-
rolled `dlopen`/`LoadLibrary`), not compile-time feature flags.  This means the
same binary works on both CUDA and non-CUDA machines: the CUDA backend simply
fails to initialize if `libcuda.so` is not present.

---

## 3. Package: `metal-compute`

### 3.1 Purpose

Layer 4B: Real Metal GPU compute on macOS.  Uses `objc-bridge` for ObjC dispatch
and the Metal framework.  On non-Apple platforms, all public functions panic with
a clear message; CI on Linux skips the tests via `#[cfg(target_vendor = "apple")]`.

### 3.2 Public types

```rust
/// A connection to one Metal GPU.
pub struct MetalDevice { /* private */ }

/// A queue for submitting work to the GPU.
pub struct MetalCommandQueue { /* private */ }

/// A contiguous region of memory accessible to both CPU and GPU.
/// On Apple Silicon (unified memory architecture), this is the same
/// physical RAM — no PCIe transfer needed.
pub struct MetalBuffer { /* private */ }

/// An MSL shader library compiled from source.
pub struct MetalLibrary { /* private */ }

/// A single compute function inside a library.
pub struct MetalFunction { /* private */ }

/// A compiled compute pipeline state (function + configuration).
pub struct MetalComputePipeline { /* private */ }
```

### 3.3 Key methods

```rust
impl MetalDevice {
    /// Open a connection to the system's default GPU.
    pub fn new() -> Result<Self, MetalError>;

    /// Allocate a shared-memory buffer (CPU + GPU accessible).
    /// On Apple Silicon, this is backed by unified RAM — no copy on upload.
    pub fn alloc(&self, len: usize) -> MetalBuffer;

    /// Allocate a buffer and immediately copy data into it.
    pub fn alloc_with_bytes(&self, data: &[u8]) -> MetalBuffer;

    /// Compile MSL compute source and return a library.
    /// Source may contain multiple `kernel` functions.
    pub fn compile(&self, source: &str) -> Result<MetalLibrary, MetalError>;

    /// Create a command queue (a channel to the GPU).
    pub fn command_queue(&self) -> MetalCommandQueue;
}

impl MetalBuffer {
    pub fn len(&self) -> usize;

    /// Read-write slice view (CPU side, Apple Silicon: also GPU-visible).
    pub fn as_slice_mut(&mut self) -> &mut [u8];

    /// Read-only slice view.
    pub fn as_slice(&self) -> &[u8];
}

impl MetalLibrary {
    pub fn function(&self, name: &str) -> Result<MetalFunction, MetalError>;
}

impl MetalDevice {
    pub fn pipeline(
        &self,
        function: &MetalFunction,
    ) -> Result<MetalComputePipeline, MetalError>;
}

impl MetalCommandQueue {
    /// Record and submit one compute dispatch.  Blocks until GPU completes.
    pub fn dispatch<F>(&self, f: F)
    where
        F: FnOnce(&mut MetalComputeEncoder);
}

pub struct MetalComputeEncoder { /* private */ }

impl MetalComputeEncoder {
    pub fn set_pipeline(&mut self, pipeline: &MetalComputePipeline);

    /// Bind a buffer at a given index (matches `[[buffer(N)]]` in MSL).
    pub fn set_buffer(&mut self, buffer: &MetalBuffer, index: u64);

    /// Set a small inline value as a buffer (avoids allocation for uniforms).
    pub fn set_bytes(&mut self, data: &[u8], index: u64);

    /// Dispatch threads.
    ///
    /// `threadgroups` — number of threadgroups in (x, y, z).
    /// `threads_per_group` — threads within each threadgroup (x, y, z).
    ///
    /// Total thread count = threadgroups.x * threads_per_group.x, etc.
    pub fn dispatch(
        &mut self,
        threadgroups:     [u32; 3],
        threads_per_group: [u32; 3],
    );
}
```

### 3.4 Memory model

On **Apple Silicon** (M1, M2, M3, …), the CPU and GPU share the same physical
RAM.  Allocating a `MetalBuffer` with `MTLResourceStorageModeShared` gives a
pointer that both CPU and GPU can dereference without any explicit copy:

```
CPU ────→ [SharedBuffer @ 0x1234] ←──── GPU shader kernel
```

This means:
- Writing to `buffer.as_slice_mut()` on CPU is immediately visible to the GPU.
- The GPU writing output to the buffer is immediately readable via
  `buffer.as_slice()` after the dispatch completes.
- There is no "upload" or "download" cost — just pointer access.

On **Intel Macs** with discrete AMD/NVIDIA GPU, the storage mode matters more;
`MTLResourceStorageModeShared` still works but goes through a PCIe copy under
the hood.  For now, we always use `Shared` for simplicity; a `Private`
(GPU-only) mode optimization can be added later.

### 3.5 Shader language: MSL

Metal Shading Language is C++ with GPU-specific qualifiers:

```metal
#include <metal_stdlib>
using namespace metal;

// A simple invert kernel: each thread processes one pixel.
kernel void invert(
    device const uint8_t* src [[buffer(0)]],
    device       uint8_t* dst [[buffer(1)]],
    constant     uint&    n   [[buffer(2)]],   // byte count
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= n) return;
    dst[gid] = 255 - src[gid];
}
```

MSL kernels are compiled at runtime by the Metal driver from source strings.
This is idiomatic Metal — Apple ships a full MSL compiler in every macOS
installation.  No offline compilation step is required (though Metal can also
precompile to `.metallib` for performance).

---

## 4. Package: `cuda-compute`

### 4.1 Purpose

Layer 4B: Real CUDA GPU compute on Linux or Windows with an NVIDIA GPU.

CUDA provides two C APIs:
- **Driver API** (`libcuda.so`, `nvcuda.dll`): low-level, explicit device context
- **Runtime API** (`libcudart.so`, `cudart.dll`): higher-level, implicit context

We use the **Driver API** for maximum control and zero dependency on NVIDIA's
Rust wrappers.  The driver API is loaded dynamically at runtime via `dlopen` on
Linux or `LoadLibrary` on Windows.

### 4.2 Dynamic loading

We do NOT link against `libcuda.so` at compile time (`#[link(name = "cuda")]`
is NOT used).  Instead:

```rust
// Hand-rolled dynamic linker
struct CudaLib {
    handle: *mut c_void,               // result of dlopen("libcuda.so.1")
    cu_init: unsafe extern "C" fn(u32) -> CUresult,
    cu_device_get: unsafe extern "C" fn(*mut CUdevice, i32) -> CUresult,
    cu_mem_alloc: unsafe extern "C" fn(*mut CUdeviceptr, usize) -> CUresult,
    cu_mem_free: unsafe extern "C" fn(CUdeviceptr) -> CUresult,
    cu_memcpy_h_to_d: unsafe extern "C" fn(CUdeviceptr, *const c_void, usize) -> CUresult,
    cu_memcpy_d_to_h: unsafe extern "C" fn(*mut c_void, CUdeviceptr, usize) -> CUresult,
    cu_module_load_data: unsafe extern "C" fn(*mut CUmodule, *const c_void) -> CUresult,
    cu_module_get_function: unsafe extern "C" fn(*mut CUfunction, CUmodule, *const c_char) -> CUresult,
    cu_launch_kernel: unsafe extern "C" fn(
        CUfunction, u32, u32, u32, u32, u32, u32,
        u32, CUstream, *mut *mut c_void, *mut *mut c_void
    ) -> CUresult,
    cu_stream_synchronize: unsafe extern "C" fn(CUstream) -> CUresult,
    // ...
}
```

`CudaLib::new()` tries to open `libcuda.so.1` on Linux or `nvcuda.dll` on
Windows.  If the library is not found, it returns `Err(CudaError::NotAvailable)`.
The `gpu-runtime` selection logic uses this to fall through to the Vulkan backend.

### 4.3 Shader compilation

CUDA kernels are CUDA C source strings:

```cuda
extern "C" __global__ void invert(
    const uint8_t* __restrict__ src,
    uint8_t* __restrict__ dst,
    uint32_t n
) {
    uint32_t gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= n) return;
    dst[gid] = 255 - src[gid];
}
```

Compilation uses NVRTC (NVIDIA Runtime Compilation), a separate library
`libnvrtc.so` / `nvrtc.dll`.  Like `libcuda.so`, NVRTC is loaded dynamically:

```rust
struct NvrtcLib {
    nvrtc_create_program: unsafe extern "C" fn(...) -> nvrtcResult,
    nvrtc_compile_program: unsafe extern "C" fn(...) -> nvrtcResult,
    nvrtc_get_ptx: unsafe extern "C" fn(...) -> nvrtcResult,
    // ...
}
```

The compilation pipeline:
```
CUDA C source string
        ↓
NvrtcLib::compile() → PTX string (assembly-like intermediate)
        ↓
CudaLib::load_module() → CUmodule (native binary for the current GPU)
        ↓
CudaLib::get_function() → CUfunction (a callable kernel handle)
```

### 4.4 Memory model

CUDA has separate CPU and GPU memory (discrete GPUs have VRAM on the card).
Transfer is explicit:

```
CPU RAM → cuMemcpyHtoD → GPU VRAM → kernel → GPU VRAM → cuMemcpyDtoH → CPU RAM
```

For NVIDIA's unified-memory (`cuMemAllocManaged`), the driver handles migrations
transparently.  We use explicit transfers for predictability.

---

## 5. Package: `vulkan-compute`

Vulkan is the cross-platform GPU API standardised by the Khronos Group.
It runs on AMD, Intel, and NVIDIA GPUs on Linux, Windows, and (via MoltenVK)
macOS.

Dynamic loading follows the same pattern as `cuda-compute`: load `libvulkan.so.1`
on Linux, `vulkan-1.dll` on Windows, or return `Err(NotAvailable)`.

Shader language: **GLSL** compute shaders compiled to **SPIR-V** at runtime
using `libshaderc` (also dynamically loaded), or offline using `glslc`.

Key Vulkan compute objects:

| Object | Purpose |
|--------|---------|
| `VkInstance` | Vulkan runtime entry point |
| `VkPhysicalDevice` | One GPU |
| `VkDevice` | Logical device (queues, pipelines) |
| `VkBuffer` | GPU-accessible memory |
| `VkDescriptorSet` | Binds buffers to shader bindings |
| `VkComputePipeline` | Compiled SPIR-V + pipeline layout |
| `VkCommandBuffer` | Recorded GPU commands |
| `VkQueue` | Submission channel |
| `VkFence` | CPU/GPU synchronization |

---

## 6. Package: `cpu-compute`

A pure-Rust backend implementing the `GpuBackend` trait.  Shader "source" is
ignored; instead, operations are registered by name and dispatched to Rust
functions.

Useful for:
- CI machines with no GPU
- Debugging (CPU is easier to step through)
- Platforms where no GPU API is available

`gpu-runtime` always registers `cpu-compute` as the final fallback; it is
never "unavailable".

---

## 7. Package: `gpu-runtime`

### 7.1 Backend detection

```rust
pub fn detect() -> Arc<dyn GpuBackend> {
    // Try Metal first (macOS)
    #[cfg(target_vendor = "apple")]
    if let Ok(d) = MetalDevice::new() {
        return Arc::new(d);
    }

    // Try CUDA (NVIDIA on Linux/Windows)
    if let Ok(d) = CudaDevice::new() {
        return Arc::new(d);
    }

    // Try Vulkan (AMD/Intel/NVIDIA on Linux/Windows)
    if let Ok(d) = VulkanDevice::new() {
        return Arc::new(d);
    }

    // Fall back to CPU
    Arc::new(CpuBackend::new())
}
```

A `once_cell::sync::Lazy<Arc<dyn GpuBackend>>` (or `std::sync::OnceLock` in
Rust 1.70+) ensures backend detection runs once per process.

### 7.2 Shader dispatch

Because each backend uses a different shader language, `gpu-runtime` takes a
`BackendShaders` struct:

```rust
pub struct BackendShaders<'a> {
    pub metal: Option<&'a str>,  // MSL source
    pub cuda:  Option<&'a str>,  // CUDA C source
    pub vulkan: Option<&'a [u8]>, // pre-compiled SPIR-V bytes
    pub cpu:   Option<fn(&[&[u8]], [u32; 3], [u32; 3])>,
}
```

`gpu-runtime` passes the appropriate shader to the selected backend.  If the
backend's shader is `None`, the dispatch returns `Err(GpuError::NoShaderForBackend)`.

---

## 8. Package: `image-gpu-core` (IMG06 revised)

Built on `gpu-runtime`.  Each operation provides shaders for all four backends.

### 8.1 Per-operation shader set

```rust
fn gpu_invert(src: &[u8], w: u32, h: u32) -> Vec<u8> {
    let rt = RUNTIME.get();
    let src_buf = rt.alloc_with_bytes(src);
    let dst_buf = rt.alloc(src.len());

    rt.dispatch(
        &INVERT_SHADER,
        &[&src_buf, &dst_buf],
        uniforms_bytes(&InvertUniforms { n: src.len() as u32 }),
        [(src.len() as u32 + 255) / 256, 1, 1],
        [256, 1, 1],
    ).expect("gpu_invert dispatch failed");

    dst_buf.to_vec()
}

static INVERT_SHADER: BackendShaders = BackendShaders {
    metal:  Some(include_str!("shaders/metal/invert.metal")),
    cuda:   Some(include_str!("shaders/cuda/invert.cu")),
    vulkan: Some(include_bytes!("shaders/vulkan/invert.spv")),
    cpu:    Some(cpu_invert),
};
```

### 8.2 Directory layout

```
code/packages/rust/image-gpu-core/
├── src/
│   ├── lib.rs
│   └── ops/
│       ├── invert.rs
│       ├── brightness.rs
│       ├── gaussian_blur.rs
│       ├── lut1d.rs
│       └── lut3d.rs
├── shaders/
│   ├── metal/       — MSL source files
│   │   ├── invert.metal
│   │   ├── point_ops.metal
│   │   ├── conv2d.metal
│   │   ├── lut1d.metal
│   │   └── lut3d.metal
│   ├── cuda/        — CUDA C source files
│   │   ├── invert.cu
│   │   ├── point_ops.cu
│   │   ├── conv2d.cu
│   │   ├── lut1d.cu
│   │   └── lut3d.cu
│   └── vulkan/      — pre-compiled SPIR-V binaries
│       ├── invert.spv
│       ├── point_ops.spv
│       ├── conv2d.spv
│       ├── lut1d.spv
│       └── lut3d.spv
```

---

## 9. Interface Summary

```
gpu-runtime:
  detect() -> Arc<dyn GpuBackend>
  GpuBackend:
    alloc(len) -> Box<dyn GpuBuffer>
    upload(buf, data)
    download(buf, out)
    compile(source, entry) -> Box<dyn GpuShader>
    dispatch(shader, buffers, uniforms, grid, block)

metal-compute (macOS only):
  MetalDevice::new() -> Result<MetalDevice, MetalError>
  MetalDevice::alloc(len) -> MetalBuffer
  MetalDevice::compile(msl_source) -> MetalLibrary
  MetalDevice::pipeline(fn) -> MetalComputePipeline
  MetalCommandQueue::dispatch(FnOnce(&mut MetalComputeEncoder))
  MetalComputeEncoder::set_pipeline(pipeline)
  MetalComputeEncoder::set_buffer(buffer, index)
  MetalComputeEncoder::set_bytes(data, index)
  MetalComputeEncoder::dispatch(threadgroups, threads_per_group)

cuda-compute (NVIDIA, dynamic-loaded):
  CudaDevice::new() -> Result<CudaDevice, CudaError>
  CudaDevice::alloc(len) -> CudaBuffer
  CudaDevice::compile_ptx(cuda_c_src) -> CudaModule
  CudaDevice::get_function(module, name) -> CudaFunction
  CudaDevice::launch(fn, buffers, uniforms, grid, block)

vulkan-compute (cross-platform, dynamic-loaded):
  VulkanDevice::new() -> Result<VulkanDevice, VulkanError>
  VulkanDevice::alloc(len) -> VulkanBuffer
  VulkanDevice::compile_spirv(spv_bytes) -> VulkanPipeline
  VulkanDevice::dispatch(pipeline, buffers, uniforms, grid, block)

image-gpu-core (uses gpu-runtime):
  gpu_invert(src, w, h) -> Vec<u8>
  gpu_brightness(src, w, h, factor) -> Vec<u8>
  gpu_colour_matrix(src, w, h, matrix_4x4) -> Vec<u8>
  gpu_gaussian_blur(src, w, h, sigma, padding) -> Vec<u8>
  gpu_apply_lut1d(src, w, h, r_lut, g_lut, b_lut) -> Vec<u8>
  gpu_apply_lut3d(src, w, h, lattice, n) -> Vec<u8>
```

---

## 10. Implementation Order

1. `metal-compute` — implement first; user's machine is macOS
2. `gpu-runtime` — implement the trait and detection logic (Metal + CPU only to start)
3. `image-gpu-core` — implement point ops with Metal shaders; add CUDA shaders in parallel
4. `cuda-compute` — implement once image-gpu-core Metal path is working
5. `vulkan-compute` — implement last; needed for Linux CI

This order means: the user gets real GPU image processing on their Mac immediately
in step 3, and CUDA/Vulkan support lands as follow-on work.
