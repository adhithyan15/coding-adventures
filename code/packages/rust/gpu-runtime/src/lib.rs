//! # gpu-runtime — Abstract GPU Compute Runtime
//!
//! G09 Layer 5: Unified interface over real GPU backends.
//!
//! On startup, `Runtime::detect()` probes available GPU backends in
//! priority order and returns the best one:
//!
//! ```text
//! Priority:
//!   1. Metal   — macOS/iOS (Apple Silicon, Intel Mac)
//!   2. CUDA    — Linux/Windows with NVIDIA GPU + CUDA toolkit
//!   3. CPU     — pure Rust fallback, always available
//! ```
//!
//! Higher-level packages (`image-gpu-core`, ML kernels, etc.) depend on
//! `gpu-runtime` and call `Runtime::global()` to get the backend.  They
//! never import `metal-compute` or `cuda-compute` directly.
//!
//! ## Shader dispatch
//!
//! Because each backend uses a different shader language, callers provide
//! per-backend shader sources in a [`Shaders`] struct:
//!
//! ```rust,ignore
//! use gpu_runtime::{Runtime, Shaders};
//!
//! static INVERT_SHADERS: Shaders = Shaders {
//!     metal: Some(include_str!("shaders/metal/invert.metal")),
//!     cuda:  Some(include_str!("shaders/cuda/invert.cu")),
//!     cpu:   Some(invert_cpu),
//! };
//!
//! fn invert_cpu(src: &[u8], dst: &mut [u8], _uniforms: &[u8]) {
//!     for (d, s) in dst.iter_mut().zip(src.iter()) {
//!         *d = 255 - s;
//!     }
//! }
//!
//! fn invert(pixels: &[u8]) -> Vec<u8> {
//!     let rt = Runtime::global();
//!     rt.run_1d(&INVERT_SHADERS, "invert", pixels, &[], pixels.len())
//!       .expect("gpu_invert failed")
//! }
//! ```
//!
//! ## Design note: per-backend shader languages
//!
//! Unlike wgpu (which uses a single shader language WGSL), `gpu-runtime`
//! accepts native shader languages for each backend:
//!
//! | Backend | Language | Why |
//! |---------|----------|-----|
//! | Metal   | MSL      | Apple's native; Metal driver compiles at runtime |
//! | CUDA    | CUDA C   | NVIDIA's native; NVRTC compiles at runtime |
//! | CPU     | Rust `fn`| No compiler needed; just a function pointer |
//!
//! This approach exposes the full expressiveness of each platform's shader
//! language, at the cost of maintaining parallel implementations per
//! operation.  The CPU implementation serves as the reference and test oracle.

pub const VERSION: &str = "0.1.0";

use std::sync::{Arc, Mutex, OnceLock};

// --------------------------------------------------------------------------
// Backend selection
// --------------------------------------------------------------------------

/// Which physical backend is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Metal,
    Cuda,
    Cpu,
}

// --------------------------------------------------------------------------
// Shaders — per-backend source bundle
// --------------------------------------------------------------------------

/// Per-backend shader sources for one operation.
///
/// Provide at minimum the CPU implementation so the runtime always has a
/// fallback.  The Metal and CUDA fields are optional — if the selected
/// backend has no shader here, `run_*` returns `Err(NoShaderForBackend)`.
pub struct Shaders {
    /// MSL compute kernel source (Metal, macOS).
    pub metal: Option<&'static str>,
    /// CUDA C kernel source (NVIDIA).
    pub cuda: Option<&'static str>,
    /// Pure-Rust CPU fallback.  Signature: `fn(src: &[u8], dst: &mut [u8], uniforms: &[u8])`.
    pub cpu: Option<fn(src: &[u8], dst: &mut [u8], uniforms: &[u8])>,
}

// --------------------------------------------------------------------------
// Error type
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GpuError {
    /// The backend supports this operation but has no shader for it.
    NoShaderForBackend { backend: BackendKind, operation: &'static str },
    /// The Metal backend returned an error.
    Metal(String),
    /// The CUDA backend returned an error.
    Cuda(String),
    /// The CPU backend encountered a logic error (should not happen).
    Cpu(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuError::NoShaderForBackend { backend, operation } =>
                write!(f, "no {backend:?} shader for operation '{operation}'"),
            GpuError::Metal(m) => write!(f, "Metal error: {m}"),
            GpuError::Cuda(m)  => write!(f, "CUDA error: {m}"),
            GpuError::Cpu(m)   => write!(f, "CPU fallback error: {m}"),
        }
    }
}

impl std::error::Error for GpuError {}

#[cfg(all(target_vendor = "apple", feature = "metal"))]
impl From<metal_compute::MetalError> for GpuError {
    fn from(e: metal_compute::MetalError) -> Self {
        GpuError::Metal(e.to_string())
    }
}

impl From<cuda_compute::CudaError> for GpuError {
    fn from(e: cuda_compute::CudaError) -> Self {
        GpuError::Cuda(e.to_string())
    }
}

// --------------------------------------------------------------------------
// Runtime
// --------------------------------------------------------------------------

/// The active GPU runtime.  Holds the selected backend.
///
/// Obtain via `Runtime::global()` (singleton) or `Runtime::detect()`
/// (creates a new instance, useful for testing).
///
/// ## Thread safety
///
/// `Runtime` is `Send + Sync`.  The `Mutex<RuntimeInner>` serialises
/// GPU dispatch: only one thread calls the backend at a time.  This is
/// sufficient for non-overlapping workloads; for concurrent dispatch,
/// create multiple `Runtime` instances or use Metal's multi-queue API.
pub struct Runtime {
    kind: BackendKind,
    inner: Mutex<RuntimeInner>,
}

/// Backend-specific state.
enum RuntimeInner {
    #[cfg(all(target_vendor = "apple", feature = "metal"))]
    Metal {
        device: metal_compute::MetalDevice,
        queue:  metal_compute::MetalCommandQueue,
    },
    Cuda {
        device: cuda_compute::CudaDevice,
    },
    Cpu,
}

static GLOBAL: OnceLock<Arc<Runtime>> = OnceLock::new();

impl Runtime {
    /// Return the process-wide singleton runtime, initialising it on the
    /// first call.  Subsequent calls return the same instance.
    pub fn global() -> Arc<Runtime> {
        GLOBAL.get_or_init(|| Arc::new(Runtime::detect())).clone()
    }

    /// Return a CPU-only runtime, bypassing backend detection.
    ///
    /// Useful in test environments where no GPU driver is accessible (sandboxes,
    /// headless CI).  The CPU path exercises the same shader dispatch code paths
    /// as Metal/CUDA — only the execution engine differs.
    pub fn cpu_only() -> Self {
        Runtime { kind: BackendKind::Cpu, inner: Mutex::new(RuntimeInner::Cpu) }
    }

    /// Probe all backends and return the best available one.
    ///
    /// Metal is tried first (macOS), then CUDA, then CPU as fallback.
    pub fn detect() -> Self {
        // 1. Metal (macOS)
        #[cfg(all(target_vendor = "apple", feature = "metal"))]
        if let Ok(device) = metal_compute::MetalDevice::new() {
            let queue = device.command_queue();
            return Runtime {
                kind: BackendKind::Metal,
                inner: Mutex::new(RuntimeInner::Metal { device, queue }),
            };
        }

        // 2. CUDA (NVIDIA on Linux/Windows)
        if let Ok(device) = cuda_compute::CudaDevice::new(0) {
            return Runtime {
                kind: BackendKind::Cuda,
                inner: Mutex::new(RuntimeInner::Cuda { device }),
            };
        }

        // 3. CPU fallback
        Runtime { kind: BackendKind::Cpu, inner: Mutex::new(RuntimeInner::Cpu) }
    }

    /// Which backend is active.
    pub fn backend(&self) -> BackendKind {
        self.kind
    }

    // ------------------------------------------------------------------
    // Dispatch helpers
    // ------------------------------------------------------------------

    /// Dispatch a 1D compute operation over `count` elements.
    ///
    /// - `shaders`  — per-backend shader bundle
    /// - `fn_name`  — name of the `kernel` / `__global__` function
    /// - `src`      — input bytes
    /// - `uniforms` — small constant data (params, sizes) passed as a buffer
    /// - `count`    — number of output bytes to produce
    ///
    /// Bindings:
    /// - Metal:  `[[buffer(0)]]` = src, `[[buffer(1)]]` = dst, `[[buffer(2)]]` = uniforms
    /// - CUDA:   arg0 = src ptr, arg1 = dst ptr, arg2 = uniforms ptr, arg3 = count
    /// - CPU:    `fn(src, dst, uniforms)`
    pub fn run_1d(
        &self,
        shaders:  &Shaders,
        fn_name:  &'static str,
        src:      &[u8],
        uniforms: &[u8],
        count:    usize,
    ) -> Result<Vec<u8>, GpuError> {
        let guard = self.inner.lock().expect("gpu-runtime mutex poisoned");
        match &*guard {
            #[cfg(all(target_vendor = "apple", feature = "metal"))]
            RuntimeInner::Metal { device, queue } => {
                let msl = shaders.metal.ok_or(GpuError::NoShaderForBackend {
                    backend: BackendKind::Metal,
                    operation: fn_name,
                })?;
                self.run_metal(device, queue, msl, fn_name, src, uniforms, count)
            }

            RuntimeInner::Cuda { device } => {
                let cu = shaders.cuda.ok_or(GpuError::NoShaderForBackend {
                    backend: BackendKind::Cuda,
                    operation: fn_name,
                })?;
                self.run_cuda(device, cu, fn_name, src, uniforms, count)
            }

            RuntimeInner::Cpu => {
                let f = shaders.cpu.ok_or(GpuError::NoShaderForBackend {
                    backend: BackendKind::Cpu,
                    operation: fn_name,
                })?;
                let mut dst = vec![0u8; count];
                f(src, &mut dst, uniforms);
                Ok(dst)
            }
        }
    }

    /// Dispatch a pixel-level compute operation where each thread handles one RGBA pixel.
    ///
    /// Like `run_1d` but the GPU launches `pixel_count` threads (one per pixel)
    /// and the output buffer is `pixel_count * 4` bytes (RGBA8 per pixel).
    /// Use this for image operations where the kernel reads/writes 4 bytes per thread.
    ///
    /// Shader conventions (thread index = pixel index):
    /// - Metal: `uint gid [[thread_position_in_grid]]` → byte offset = `gid * 4`
    /// - CUDA:  `uint gid = blockIdx.x * blockDim.x + threadIdx.x` → same formula
    /// - CPU:   `fn(src, dst, uniforms)` where src/dst are full RGBA byte buffers
    pub fn run_pixels(
        &self,
        shaders:     &Shaders,
        fn_name:     &'static str,
        src:         &[u8],
        uniforms:    &[u8],
        pixel_count: usize,
    ) -> Result<Vec<u8>, GpuError> {
        // Validate pixel_count * 4 fits in usize before any backend allocates
        // memory.  This is a public API so callers outside image-gpu-core (which
        // already uses checked_mul) could pass an oversized count.
        let byte_count = pixel_count
            .checked_mul(4)
            .ok_or_else(|| GpuError::Cpu(format!(
                "pixel_count {pixel_count} overflows usize when multiplied by 4"
            )))?;

        let guard = self.inner.lock().expect("gpu-runtime mutex poisoned");
        match &*guard {
            #[cfg(all(target_vendor = "apple", feature = "metal"))]
            RuntimeInner::Metal { device, queue } => {
                let msl = shaders.metal.ok_or(GpuError::NoShaderForBackend {
                    backend: BackendKind::Metal,
                    operation: fn_name,
                })?;
                self.run_metal_pixels(device, queue, msl, fn_name, src, uniforms, pixel_count, byte_count)
            }

            RuntimeInner::Cuda { device } => {
                let cu = shaders.cuda.ok_or(GpuError::NoShaderForBackend {
                    backend: BackendKind::Cuda,
                    operation: fn_name,
                })?;
                self.run_cuda_pixels(device, cu, fn_name, src, uniforms, pixel_count, byte_count)
            }

            RuntimeInner::Cpu => {
                let f = shaders.cpu.ok_or(GpuError::NoShaderForBackend {
                    backend: BackendKind::Cpu,
                    operation: fn_name,
                })?;
                let mut dst = vec![0u8; byte_count];
                f(src, &mut dst, uniforms);
                Ok(dst)
            }
        }
    }

    // ------------------------------------------------------------------
    // Metal dispatch
    // ------------------------------------------------------------------

    #[cfg(all(target_vendor = "apple", feature = "metal"))]
    fn run_metal(
        &self,
        device:   &metal_compute::MetalDevice,
        queue:    &metal_compute::MetalCommandQueue,
        msl:      &str,
        fn_name:  &str,
        src:      &[u8],
        uniforms: &[u8],
        count:    usize,
    ) -> Result<Vec<u8>, GpuError> {
        let src_buf  = device.alloc_with_bytes(src)?;
        let dst_buf  = device.alloc(count)?;
        let unif_buf = if uniforms.is_empty() {
            device.alloc(4)? // 4 zero bytes (uniforms slot required by shader)
        } else {
            device.alloc_with_bytes(uniforms)?
        };

        let lib      = device.compile(msl)?;
        let func     = lib.function(fn_name)?;
        let pipeline = device.pipeline(&func)?;
        let tpg      = pipeline.preferred_threads_1d();

        queue.dispatch(|enc| {
            enc.set_pipeline(&pipeline);
            enc.set_buffer(&src_buf,  0);
            enc.set_buffer(&dst_buf,  1);
            enc.set_buffer(&unif_buf, 2);
            enc.dispatch_threads_1d(count as u32, tpg);
        });

        Ok(dst_buf.to_vec())
    }

    #[cfg(all(target_vendor = "apple", feature = "metal"))]
    fn run_metal_pixels(
        &self,
        device:      &metal_compute::MetalDevice,
        queue:       &metal_compute::MetalCommandQueue,
        msl:         &str,
        fn_name:     &str,
        src:         &[u8],
        uniforms:    &[u8],
        pixel_count: usize,
        byte_count:  usize,
    ) -> Result<Vec<u8>, GpuError> {
        let src_buf  = device.alloc_with_bytes(src)?;
        let dst_buf  = device.alloc(byte_count)?;
        let unif_buf = if uniforms.is_empty() {
            device.alloc(4)?
        } else {
            device.alloc_with_bytes(uniforms)?
        };

        let lib      = device.compile(msl)?;
        let func     = lib.function(fn_name)?;
        let pipeline = device.pipeline(&func)?;
        let tpg      = pipeline.preferred_threads_1d();

        queue.dispatch(|enc| {
            enc.set_pipeline(&pipeline);
            enc.set_buffer(&src_buf,  0);
            enc.set_buffer(&dst_buf,  1);
            enc.set_buffer(&unif_buf, 2);
            enc.dispatch_threads_1d(pixel_count as u32, tpg);
        });

        Ok(dst_buf.to_vec())
    }

    // ------------------------------------------------------------------
    // CUDA dispatch
    // ------------------------------------------------------------------

    fn run_cuda(
        &self,
        device:   &cuda_compute::CudaDevice,
        cuda_c:   &str,
        fn_name:  &str,
        src:      &[u8],
        uniforms: &[u8],
        count:    usize,
    ) -> Result<Vec<u8>, GpuError> {
        let src_buf = device.alloc_with_bytes(src)?;
        let dst_buf = device.alloc(count)?;

        // Optional uniforms buffer — only allocated when uniforms are non-empty,
        // because CUDA kernel signatures differ:
        //   no-uniform ops: (src, dst, n)
        //   uniform ops:    (src, dst, uni, n)
        let unif_buf = if uniforms.is_empty() {
            None
        } else {
            Some(device.alloc_with_bytes(uniforms)?)
        };

        let module   = device.compile(cuda_c)?;
        let function = module.function(fn_name)?;

        let n          = count as u32;
        let block_size = 256u32;
        let grid_size  = (n + block_size - 1) / block_size;

        // Pass the actual CUdeviceptr values, not lengths.
        let mut src_ptr = src_buf.device_ptr();
        let mut dst_ptr = dst_buf.device_ptr();
        let mut n_arg   = n;

        if let Some(ref ub) = unif_buf {
            let mut uni_ptr = ub.device_ptr();
            let mut args: [*mut std::ffi::c_void; 4] = [
                &mut src_ptr as *mut _ as *mut std::ffi::c_void,
                &mut dst_ptr as *mut _ as *mut std::ffi::c_void,
                &mut uni_ptr as *mut _ as *mut std::ffi::c_void,
                &mut n_arg   as *mut _ as *mut std::ffi::c_void,
            ];
            device.launch(&function, [grid_size, 1, 1], [block_size, 1, 1], &mut args)?;
        } else {
            let mut args: [*mut std::ffi::c_void; 3] = [
                &mut src_ptr as *mut _ as *mut std::ffi::c_void,
                &mut dst_ptr as *mut _ as *mut std::ffi::c_void,
                &mut n_arg   as *mut _ as *mut std::ffi::c_void,
            ];
            device.launch(&function, [grid_size, 1, 1], [block_size, 1, 1], &mut args)?;
        }

        device.synchronize()?;
        device.download(&dst_buf).map_err(GpuError::from)
    }

    fn run_cuda_pixels(
        &self,
        device:      &cuda_compute::CudaDevice,
        cuda_c:      &str,
        fn_name:     &str,
        src:         &[u8],
        uniforms:    &[u8],
        pixel_count: usize,
        byte_count:  usize,
    ) -> Result<Vec<u8>, GpuError> {
        let src_buf = device.alloc_with_bytes(src)?;
        let dst_buf = device.alloc(byte_count)?;

        let unif_buf = if uniforms.is_empty() {
            None
        } else {
            Some(device.alloc_with_bytes(uniforms)?)
        };

        let module   = device.compile(cuda_c)?;
        let function = module.function(fn_name)?;

        let n          = pixel_count as u32;
        let block_size = 256u32;
        let grid_size  = (n + block_size - 1) / block_size;

        let mut src_ptr = src_buf.device_ptr();
        let mut dst_ptr = dst_buf.device_ptr();
        let mut n_arg   = n;

        if let Some(ref ub) = unif_buf {
            let mut uni_ptr = ub.device_ptr();
            let mut args: [*mut std::ffi::c_void; 4] = [
                &mut src_ptr as *mut _ as *mut std::ffi::c_void,
                &mut dst_ptr as *mut _ as *mut std::ffi::c_void,
                &mut uni_ptr as *mut _ as *mut std::ffi::c_void,
                &mut n_arg   as *mut _ as *mut std::ffi::c_void,
            ];
            device.launch(&function, [grid_size, 1, 1], [block_size, 1, 1], &mut args)?;
        } else {
            let mut args: [*mut std::ffi::c_void; 3] = [
                &mut src_ptr as *mut _ as *mut std::ffi::c_void,
                &mut dst_ptr as *mut _ as *mut std::ffi::c_void,
                &mut n_arg   as *mut _ as *mut std::ffi::c_void,
            ];
            device.launch(&function, [grid_size, 1, 1], [block_size, 1, 1], &mut args)?;
        }

        device.synchronize()?;
        device.download(&dst_buf).map_err(GpuError::from)
    }
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Detect should always succeed (CPU fallback ensures it).
    /// Ignored by default because `detect()` probes Metal on macOS, which
    /// requires IOKit GPU access unavailable in sandboxed test environments.
    /// Run with `cargo test -- --ignored` on a machine with a real GPU.
    #[test]
    #[ignore = "requires GPU driver access (Metal/CUDA)"]
    fn detect_succeeds() {
        let rt = Runtime::detect();
        // On macOS: Metal.  On NVIDIA Linux: CUDA.  Elsewhere: CPU.
        eprintln!("Detected backend: {:?}", rt.backend());
    }

    /// CPU fallback with a simple invert shader.
    #[test]
    fn cpu_fallback_invert() {
        fn cpu_invert(src: &[u8], dst: &mut [u8], _uniforms: &[u8]) {
            for (d, s) in dst.iter_mut().zip(src.iter()) {
                *d = 255 - s;
            }
        }

        let shaders = Shaders {
            metal: None,
            cuda: None,
            cpu: Some(cpu_invert),
        };

        // Force CPU backend.
        let rt = Runtime { kind: BackendKind::Cpu, inner: Mutex::new(RuntimeInner::Cpu) };

        let src: Vec<u8> = (0u8..=255).collect();
        let expected: Vec<u8> = src.iter().map(|&b| 255 - b).collect();

        let result = rt.run_1d(&shaders, "invert", &src, &[], src.len()).unwrap();
        assert_eq!(result, expected);
    }

    /// `global()` returns the same instance each time.
    /// Ignored by default for the same reason as `detect_succeeds`.
    #[test]
    #[ignore = "requires GPU driver access (Metal/CUDA)"]
    fn global_is_singleton() {
        let a = Runtime::global();
        let b = Runtime::global();
        // Same Arc means same allocation.
        assert!(Arc::ptr_eq(&a, &b));
    }
}
