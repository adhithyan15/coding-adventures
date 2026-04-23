//! # cuda-compute — Real CUDA GPU Compute via Dynamic Loading
//!
//! G09 Layer 4B: This crate wraps NVIDIA's CUDA Driver API for GPU compute,
//! loaded at **runtime** with no link-time dependency on CUDA libraries.
//!
//! ## Why dynamic loading?
//!
//! Linking against `libcuda.so` at compile time would make the binary fail to
//! start on machines without CUDA installed.  Instead, we `dlopen("libcuda.so.1")`
//! at runtime: if it is not present, `CudaDevice::new()` returns
//! `Err(CudaError::NotAvailable)` and the caller can fall back to another
//! backend.  The binary compiles and runs on any Linux/Windows machine.
//!
//! ## APIs used
//!
//! CUDA has two layers:
//!
//! - **Driver API** (`libcuda.so`): low-level, explicit context management.
//!   Symbols: `cuInit`, `cuDeviceGet`, `cuCtxCreate`, `cuMemAlloc`, …
//! - **Runtime API** (`libcudart.so`): higher-level, implicit context.
//!   Symbols: `cudaMalloc`, `cudaMemcpy`, …
//!
//! We use the **Driver API** because it is more stable across CUDA versions
//! and doesn't require `libcudart.so`.
//!
//! ## Shader compilation (NVRTC)
//!
//! NVIDIA Runtime Compilation (`libnvrtc.so`) compiles CUDA C source strings
//! to PTX (NVIDIA's intermediate assembly).  PTX is then loaded into a CUDA
//! module, which the Driver API can execute.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use cuda_compute::{CudaDevice, CudaError};
//!
//! const CUDA_C: &str = r#"
//! extern "C" __global__ void invert(
//!     const unsigned char* src,
//!     unsigned char* dst,
//!     unsigned int n
//! ) {
//!     unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
//!     if (gid >= n) return;
//!     dst[gid] = 255 - src[gid];
//! }
//! "#;
//!
//! fn invert(pixels: &[u8]) -> Result<Vec<u8>, CudaError> {
//!     let device = CudaDevice::new(0)?;  // GPU index 0
//!     let src = device.alloc_with_bytes(pixels)?;
//!     let dst = device.alloc(pixels.len())?;
//!
//!     let module = device.compile(CUDA_C)?;
//!     let kernel = module.function("invert")?;
//!
//!     let n = pixels.len() as u32;
//!     let block = 256u32;
//!     let grid  = (n + block - 1) / block;
//!
//!     device.launch(&kernel, [grid, 1, 1], [block, 1, 1], &[
//!         src.as_kernel_arg(),
//!         dst.as_kernel_arg(),
//!         (&n as *const u32 as *mut std::ffi::c_void, std::mem::size_of::<u32>()),
//!     ])?;
//!     device.synchronize()?;
//!
//!     device.download(&dst)
//! }
//! ```

pub const VERSION: &str = "0.1.0";

use std::ffi::{c_void, c_char, CStr, CString};

// --------------------------------------------------------------------------
// Error type
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum CudaError {
    /// CUDA is not available on this machine (no GPU, driver not installed,
    /// or `libcuda.so` not found).
    NotAvailable,
    /// A CUDA Driver API call returned a non-zero result code.
    DriverError { code: i32, message: String },
    /// NVRTC (compiler) not found — `libnvrtc.so` is missing.
    NvrtcNotFound,
    /// NVRTC compilation of CUDA C source failed.
    CompileFailed(String),
    /// A named kernel was not found in the compiled module.
    FunctionNotFound(String),
    /// `dlopen` failed to load the specified library.
    DlopenFailed(String),
}

impl std::fmt::Display for CudaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CudaError::NotAvailable         => write!(f, "CUDA is not available on this system"),
            CudaError::DriverError { code, message } =>
                write!(f, "CUDA driver error {code}: {message}"),
            CudaError::NvrtcNotFound        => write!(f, "libnvrtc not found — install CUDA toolkit"),
            CudaError::CompileFailed(msg)   => write!(f, "NVRTC compilation failed: {msg}"),
            CudaError::FunctionNotFound(n)  => write!(f, "CUDA kernel '{n}' not found in module"),
            CudaError::DlopenFailed(lib)    => write!(f, "dlopen failed for {lib}"),
        }
    }
}

impl std::error::Error for CudaError {}

// --------------------------------------------------------------------------
// CUDA Driver API opaque types
// --------------------------------------------------------------------------
//
// CUDA uses C integer typedefs and opaque pointer handles.  We define
// them here to match the CUDA Driver API ABI on 64-bit platforms.

/// CUDA result code.  0 = success.
pub type CUresult = i32;

/// Device handle (integer index).
pub type CUdevice = i32;

/// Context handle (opaque pointer).
pub type CUcontext = *mut c_void;

/// Module handle (opaque pointer).
pub type CUmodule = *mut c_void;

/// Kernel function handle (opaque pointer).
pub type CUfunction = *mut c_void;

/// Device memory pointer.  On 64-bit CUDA, this is `unsigned long long`.
pub type CUdeviceptr = u64;

/// CUDA stream handle (opaque pointer).
pub type CUstream = *mut c_void;

/// Null CUDA stream (default stream, synchronizes all operations).
pub const CU_STREAM_LEGACY: CUstream = 1usize as CUstream;

// CUDA context creation flags.
pub const CU_CTX_SCHED_AUTO: u32 = 0;

// cuMemcpy direction constants.
pub const CU_MEMCPY_HOST_TO_DEVICE: u32 = 1;
pub const CU_MEMCPY_DEVICE_TO_HOST: u32 = 2;

// --------------------------------------------------------------------------
// NVRTC opaque types
// --------------------------------------------------------------------------

pub type NvrtcProgram = *mut c_void;

/// NVRTC result code.  0 = success.
pub type NvrtcResult = i32;

// --------------------------------------------------------------------------
// Dynamic library loading (dlopen / LoadLibrary)
// --------------------------------------------------------------------------

#[cfg(unix)]
mod dynlib {
    use super::*;

    extern "C" {
        fn dlopen(filename: *const c_char, flag: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> i32;
        #[allow(dead_code)]
        fn dlerror() -> *mut c_char;
    }

    pub const RTLD_NOW: i32 = 2;
    pub const RTLD_GLOBAL: i32 = 8;

    pub struct DynLib(*mut c_void);

    impl Drop for DynLib {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { dlclose(self.0) };
            }
        }
    }

    impl DynLib {
        pub fn open(name: &str) -> Option<Self> {
            let c_name = CString::new(name).ok()?;
            let handle = unsafe { dlopen(c_name.as_ptr(), RTLD_NOW | RTLD_GLOBAL) };
            if handle.is_null() {
                None
            } else {
                Some(DynLib(handle))
            }
        }

        pub fn symbol<T>(&self, name: &str) -> Option<T> {
            // T must be pointer-sized and pointer-aligned for transmute_copy to be sound.
            assert_eq!(
                std::mem::size_of::<T>(),
                std::mem::size_of::<*mut c_void>(),
                "DynLib::symbol: T must be pointer-sized"
            );
            assert_eq!(
                std::mem::align_of::<T>(),
                std::mem::align_of::<*mut c_void>(),
                "DynLib::symbol: T must have pointer alignment"
            );
            let c_name = CString::new(name).ok()?;
            let sym = unsafe { dlsym(self.0, c_name.as_ptr()) };
            if sym.is_null() {
                None
            } else {
                // SAFETY: caller is responsible for ensuring T matches the symbol's type.
                Some(unsafe { std::mem::transmute_copy(&sym) })
            }
        }
    }
}

#[cfg(windows)]
mod dynlib {
    use super::*;
    use std::os::windows::ffi::OsStrExt;

    extern "system" {
        fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
        fn FreeLibrary(hModule: *mut c_void) -> i32;
        fn GetProcAddress(hModule: *mut c_void, lpProcName: *const c_char) -> *mut c_void;
    }

    pub struct DynLib(*mut c_void);

    impl Drop for DynLib {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { FreeLibrary(self.0) };
            }
        }
    }

    impl DynLib {
        pub fn open(name: &str) -> Option<Self> {
            let wide: Vec<u16> = std::ffi::OsStr::new(name)
                .encode_wide().chain(Some(0)).collect();
            let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
            if handle.is_null() { None } else { Some(DynLib(handle)) }
        }

        pub fn symbol<T>(&self, name: &str) -> Option<T> {
            // T must be pointer-sized and pointer-aligned — see Unix impl for rationale.
            assert_eq!(
                std::mem::size_of::<T>(),
                std::mem::size_of::<*mut c_void>(),
                "DynLib::symbol: T must be pointer-sized"
            );
            assert_eq!(
                std::mem::align_of::<T>(),
                std::mem::align_of::<*mut c_void>(),
                "DynLib::symbol: T must have pointer alignment"
            );
            let c_name = CString::new(name).ok()?;
            let sym = unsafe { GetProcAddress(self.0, c_name.as_ptr()) };
            if sym.is_null() { None }
            else { Some(unsafe { std::mem::transmute_copy(&sym) }) }
        }
    }
}

// On macOS: CUDA is not available (Apple dropped NVIDIA drivers in 2019).
// We still compile the crate but every constructor returns NotAvailable.
#[cfg(not(any(unix, windows)))]
mod dynlib {
    pub struct DynLib;
    impl DynLib {
        pub fn open(_name: &str) -> Option<Self> { None }
        pub fn symbol<T>(&self, _name: &str) -> Option<T> { None }
    }
}

// --------------------------------------------------------------------------
// CudaLib — loaded CUDA Driver API functions
// --------------------------------------------------------------------------

type FnCuInit        = unsafe extern "C" fn(flags: u32) -> CUresult;
type FnCuDeviceGet   = unsafe extern "C" fn(device: *mut CUdevice, ordinal: i32) -> CUresult;
type FnCuDeviceGetCount = unsafe extern "C" fn(count: *mut i32) -> CUresult;
type FnCuCtxCreate   = unsafe extern "C" fn(pctx: *mut CUcontext, flags: u32, dev: CUdevice) -> CUresult;
type FnCuCtxDestroy  = unsafe extern "C" fn(ctx: CUcontext) -> CUresult;
type FnCuMemAlloc    = unsafe extern "C" fn(dptr: *mut CUdeviceptr, byte_size: usize) -> CUresult;
type FnCuMemFree     = unsafe extern "C" fn(dptr: CUdeviceptr) -> CUresult;
type FnCuMemcpyHtoD  = unsafe extern "C" fn(dst: CUdeviceptr, src: *const c_void, byte_count: usize) -> CUresult;
type FnCuMemcpyDtoH  = unsafe extern "C" fn(dst: *mut c_void, src: CUdeviceptr, byte_count: usize) -> CUresult;
type FnCuModuleLoadData   = unsafe extern "C" fn(module: *mut CUmodule, image: *const c_void) -> CUresult;
type FnCuModuleUnload     = unsafe extern "C" fn(module: CUmodule) -> CUresult;
type FnCuModuleGetFunction = unsafe extern "C" fn(hfunc: *mut CUfunction, hmod: CUmodule, name: *const c_char) -> CUresult;
type FnCuLaunchKernel     = unsafe extern "C" fn(
    f: CUfunction,
    grid_x: u32, grid_y: u32, grid_z: u32,
    block_x: u32, block_y: u32, block_z: u32,
    shared_mem: u32,
    stream: CUstream,
    kernel_params: *mut *mut c_void,
    extra: *mut *mut c_void,
) -> CUresult;
type FnCuCtxSynchronize  = unsafe extern "C" fn() -> CUresult;
type FnCuCtxSetCurrent   = unsafe extern "C" fn(ctx: CUcontext) -> CUresult;
type FnCuGetErrorName    = unsafe extern "C" fn(error: CUresult, pstr: *mut *const c_char) -> CUresult;

struct CudaLib {
    _handle: dynlib::DynLib,
    cu_init:              FnCuInit,
    cu_device_get:        FnCuDeviceGet,
    cu_device_get_count:  FnCuDeviceGetCount,
    cu_ctx_create:        FnCuCtxCreate,
    cu_ctx_destroy:       FnCuCtxDestroy,
    cu_ctx_set_current:   FnCuCtxSetCurrent,
    cu_mem_alloc:         FnCuMemAlloc,
    cu_mem_free:          FnCuMemFree,
    cu_memcpy_h_to_d:     FnCuMemcpyHtoD,
    cu_memcpy_d_to_h:     FnCuMemcpyDtoH,
    cu_module_load_data:  FnCuModuleLoadData,
    cu_module_unload:     FnCuModuleUnload,
    cu_module_get_function: FnCuModuleGetFunction,
    cu_launch_kernel:     FnCuLaunchKernel,
    cu_ctx_synchronize:   FnCuCtxSynchronize,
    cu_get_error_name:    FnCuGetErrorName,
}

impl CudaLib {
    fn load() -> Option<Self> {
        // Try platform-appropriate library names.
        #[cfg(unix)]
        let names = &["libcuda.so.1", "libcuda.so"];
        #[cfg(windows)]
        let names = &["nvcuda.dll"];
        #[cfg(not(any(unix, windows)))]
        let names: &[&str] = &[];

        let handle = names.iter().find_map(|n| dynlib::DynLib::open(n))?;

        macro_rules! sym {
            ($name:literal) => {
                handle.symbol($name)?
            };
        }

        Some(CudaLib {
            cu_init:               sym!("cuInit"),
            cu_device_get:         sym!("cuDeviceGet"),
            cu_device_get_count:   sym!("cuDeviceGetCount"),
            cu_ctx_create:         sym!("cuCtxCreate_v2"),
            cu_ctx_destroy:        sym!("cuCtxDestroy_v2"),
            cu_ctx_set_current:    sym!("cuCtxSetCurrent"),
            cu_mem_alloc:          sym!("cuMemAlloc_v2"),
            cu_mem_free:           sym!("cuMemFree_v2"),
            cu_memcpy_h_to_d:      sym!("cuMemcpyHtoD_v2"),
            cu_memcpy_d_to_h:      sym!("cuMemcpyDtoH_v2"),
            cu_module_load_data:   sym!("cuModuleLoadData"),
            cu_module_unload:      sym!("cuModuleUnload"),
            cu_module_get_function: sym!("cuModuleGetFunction"),
            cu_launch_kernel:      sym!("cuLaunchKernel"),
            cu_ctx_synchronize:    sym!("cuCtxSynchronize"),
            cu_get_error_name:     sym!("cuGetErrorName"),
            _handle: handle,
        })
    }

    fn check(&self, result: CUresult) -> Result<(), CudaError> {
        if result == 0 {
            return Ok(());
        }
        // Copy the error name string into owned memory before returning.
        // The C string points into the dynamically-loaded driver; holding a
        // borrow across a library unload would be a dangling pointer.
        // Check cu_get_error_name's own return value before dereferencing ptr:
        // if the query itself fails (non-zero), ptr may remain null or invalid.
        let message = unsafe {
            let mut ptr: *const c_char = std::ptr::null();
            let name_r = (self.cu_get_error_name)(result, &mut ptr);
            if name_r == 0 && !ptr.is_null() {
                CStr::from_ptr(ptr).to_str().unwrap_or("unreadable").to_owned()
            } else {
                format!("unknown CUDA error (code {result})")
            }
        };
        Err(CudaError::DriverError { code: result, message })
    }
}

// --------------------------------------------------------------------------
// NvrtcLib — loaded NVRTC compiler functions
// --------------------------------------------------------------------------

type FnNvrtcCreateProgram  = unsafe extern "C" fn(
    prog: *mut NvrtcProgram,
    src: *const c_char,
    name: *const c_char,
    num_headers: i32,
    headers: *const *const c_char,
    include_names: *const *const c_char,
) -> NvrtcResult;
type FnNvrtcCompileProgram = unsafe extern "C" fn(prog: NvrtcProgram, num_options: i32, options: *const *const c_char) -> NvrtcResult;
type FnNvrtcGetPTXSize     = unsafe extern "C" fn(prog: NvrtcProgram, ptx_size_ret: *mut usize) -> NvrtcResult;
type FnNvrtcGetPTX         = unsafe extern "C" fn(prog: NvrtcProgram, ptx: *mut c_char) -> NvrtcResult;
type FnNvrtcGetProgramLogSize = unsafe extern "C" fn(prog: NvrtcProgram, log_size: *mut usize) -> NvrtcResult;
type FnNvrtcGetProgramLog  = unsafe extern "C" fn(prog: NvrtcProgram, log: *mut c_char) -> NvrtcResult;
type FnNvrtcDestroyProgram = unsafe extern "C" fn(prog: *mut NvrtcProgram) -> NvrtcResult;

struct NvrtcLib {
    _handle: dynlib::DynLib,
    nvrtc_create_program:   FnNvrtcCreateProgram,
    nvrtc_compile_program:  FnNvrtcCompileProgram,
    nvrtc_get_ptx_size:     FnNvrtcGetPTXSize,
    nvrtc_get_ptx:          FnNvrtcGetPTX,
    nvrtc_get_program_log_size: FnNvrtcGetProgramLogSize,
    nvrtc_get_program_log:  FnNvrtcGetProgramLog,
    nvrtc_destroy_program:  FnNvrtcDestroyProgram,
}

impl NvrtcLib {
    fn load() -> Option<Self> {
        #[cfg(unix)]
        let names = &["libnvrtc.so.1", "libnvrtc.so", "libnvrtc-builtins.so.1"];
        #[cfg(windows)]
        let names = &["nvrtc.dll", "nvrtc64_120_0.dll"];
        #[cfg(not(any(unix, windows)))]
        let names: &[&str] = &[];

        let handle = names.iter().find_map(|n| dynlib::DynLib::open(n))?;

        macro_rules! sym {
            ($name:literal) => { handle.symbol($name)? };
        }

        Some(NvrtcLib {
            nvrtc_create_program:       sym!("nvrtcCreateProgram"),
            nvrtc_compile_program:      sym!("nvrtcCompileProgram"),
            nvrtc_get_ptx_size:         sym!("nvrtcGetPTXSize"),
            nvrtc_get_ptx:              sym!("nvrtcGetPTX"),
            nvrtc_get_program_log_size: sym!("nvrtcGetProgramLogSize"),
            nvrtc_get_program_log:      sym!("nvrtcGetProgramLog"),
            nvrtc_destroy_program:      sym!("nvrtcDestroyProgram"),
            _handle: handle,
        })
    }

    /// Compile CUDA C source to a PTX string.
    fn compile(&self, source: &str) -> Result<String, CudaError> {
        unsafe {
            let c_src = CString::new(source)
                .map_err(|_| CudaError::CompileFailed("source contains interior NUL byte".to_string()))?;
            let c_name = CString::new("kernel.cu").expect("static literal is valid CString");
            let mut prog: NvrtcProgram = std::ptr::null_mut();

            let r = (self.nvrtc_create_program)(
                &mut prog, c_src.as_ptr(), c_name.as_ptr(),
                0, std::ptr::null(), std::ptr::null(),
            );
            if r != 0 {
                return Err(CudaError::CompileFailed(format!("nvrtcCreateProgram failed: {r}")));
            }

            let compile_r = (self.nvrtc_compile_program)(prog, 0, std::ptr::null());

            // Always retrieve the log, even on success (it may contain warnings).
            // Check return values: if size query fails, skip log retrieval rather
            // than allocating a 0-byte buffer and letting nvrtcGetProgramLog write
            // an unknown number of bytes into it (heap buffer overflow risk).
            // Cap allocation at 16 MiB — NVRTC log sizes beyond this indicate a
            // driver bug or corrupted state; honouring them would be a DoS vector.
            const MAX_NVRTC_BYTES: usize = 16 * 1024 * 1024;
            let mut log_size: usize = 0;
            let log_size_r = (self.nvrtc_get_program_log_size)(prog, &mut log_size);
            let log = if log_size_r == 0 && log_size > 0 && log_size <= MAX_NVRTC_BYTES {
                let mut log_buf = vec![0u8; log_size];
                let log_r = (self.nvrtc_get_program_log)(prog, log_buf.as_mut_ptr() as *mut c_char);
                if log_r == 0 {
                    String::from_utf8_lossy(&log_buf).trim_end_matches('\0').to_string()
                } else {
                    format!("(nvrtcGetProgramLog failed: {log_r})")
                }
            } else {
                String::new()
            };

            if compile_r != 0 {
                (self.nvrtc_destroy_program)(&mut prog);
                return Err(CudaError::CompileFailed(if log.is_empty() {
                    format!("nvrtcCompileProgram failed: {compile_r}")
                } else {
                    log
                }));
            }

            let mut ptx_size: usize = 0;
            let ptx_size_r = (self.nvrtc_get_ptx_size)(prog, &mut ptx_size);
            if ptx_size_r != 0 {
                (self.nvrtc_destroy_program)(&mut prog);
                return Err(CudaError::CompileFailed(
                    format!("nvrtcGetPTXSize failed: {ptx_size_r}")
                ));
            }
            if ptx_size > MAX_NVRTC_BYTES {
                (self.nvrtc_destroy_program)(&mut prog);
                return Err(CudaError::CompileFailed(
                    format!("nvrtcGetPTXSize returned implausibly large PTX ({ptx_size} bytes)")
                ));
            }
            let mut ptx = vec![0u8; ptx_size];
            let ptx_r = (self.nvrtc_get_ptx)(prog, ptx.as_mut_ptr() as *mut c_char);
            (self.nvrtc_destroy_program)(&mut prog);
            if ptx_r != 0 {
                return Err(CudaError::CompileFailed(
                    format!("nvrtcGetPTX failed: {ptx_r}")
                ));
            }

            let ptx_str = String::from_utf8_lossy(&ptx).trim_end_matches('\0').to_string();
            Ok(ptx_str)
        }
    }
}

// --------------------------------------------------------------------------
// Public API
// --------------------------------------------------------------------------

// CUDA Driver API contexts are associated with the creating thread.  We
// implement Send so the device can be moved to another thread; callers
// (e.g. gpu-runtime wrapping it in a Mutex) are responsible for ensuring
// serial access.  Sync is intentionally NOT implemented.
unsafe impl Send for CudaDevice {}

/// A connection to one CUDA-capable GPU, plus a CUDA context.
///
/// Manages the `CUcontext` lifecycle.  The CUDA Driver API requires a current
/// context on the calling thread for all GPU operations; `CudaDevice` binds
/// the context on creation and is single-threaded (not `Send`).
///
/// Drop releases the context.
pub struct CudaDevice {
    cuda: std::sync::Arc<CudaLib>,
    nvrtc: std::sync::Arc<NvrtcLib>,
    ctx: CUcontext,
}

impl Drop for CudaDevice {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            unsafe { (self.cuda.cu_ctx_destroy)(self.ctx) };
        }
    }
}

impl CudaDevice {
    /// Open a CUDA context on GPU `device_index` (0-based).
    ///
    /// Dynamically loads `libcuda.so` and `libnvrtc.so`.  Returns
    /// `Err(CudaError::NotAvailable)` if CUDA is not installed, or
    /// `Err(CudaError::NvrtcNotFound)` if only the driver is present
    /// but not the toolkit.
    pub fn new(device_index: i32) -> Result<Self, CudaError> {
        let cuda = CudaLib::load().ok_or(CudaError::NotAvailable)?;
        let nvrtc = NvrtcLib::load().ok_or(CudaError::NvrtcNotFound)?;

        unsafe {
            cuda.check((cuda.cu_init)(0))?;

            let mut dev: CUdevice = 0;
            cuda.check((cuda.cu_device_get)(&mut dev, device_index))?;

            let mut ctx: CUcontext = std::ptr::null_mut();
            cuda.check((cuda.cu_ctx_create)(&mut ctx, CU_CTX_SCHED_AUTO, dev))?;

            Ok(CudaDevice {
                cuda: std::sync::Arc::new(cuda),
                nvrtc: std::sync::Arc::new(nvrtc),
                ctx,
            })
        }
    }

    /// Return the number of CUDA-capable GPUs in the system.
    ///
    /// `CudaDevice::new()` must have been called at least once to initialize
    /// the driver.
    pub fn device_count() -> Result<i32, CudaError> {
        let cuda = CudaLib::load().ok_or(CudaError::NotAvailable)?;
        unsafe {
            cuda.check((cuda.cu_init)(0))?;
            let mut count: i32 = 0;
            cuda.check((cuda.cu_device_get_count)(&mut count))?;
            Ok(count)
        }
    }

    /// Make this device's context current on the calling thread.
    ///
    /// CUDA Driver API contexts are created on and bound to a specific thread.
    /// When the device is moved to another thread (via the `Send` impl and a
    /// `Mutex`), callers must rebind the context before issuing any CUDA work.
    /// This method does that atomically.  `gpu-runtime` calls it through the
    /// Mutex, so only one thread rebinds at a time.
    fn bind_ctx(&self) -> Result<(), CudaError> {
        unsafe { self.cuda.check((self.cuda.cu_ctx_set_current)(self.ctx)) }
    }

    /// Allocate `len` bytes of device memory.
    pub fn alloc(&self, len: usize) -> Result<CudaBuffer, CudaError> {
        if len == 0 {
            return Err(CudaError::DriverError {
                code: -1,
                message: "alloc: zero-length allocation is not permitted".to_string(),
            });
        }
        self.bind_ctx()?;
        let mut dptr: CUdeviceptr = 0;
        unsafe { self.cuda.check((self.cuda.cu_mem_alloc)(&mut dptr, len))? };
        Ok(CudaBuffer {
            cuda: self.cuda.clone(),
            dptr,
            len,
        })
    }

    /// Allocate device memory and upload `data` from the CPU.
    pub fn alloc_with_bytes(&self, data: &[u8]) -> Result<CudaBuffer, CudaError> {
        // bind_ctx is called inside alloc() and upload().
        let buf = self.alloc(data.len())?;
        self.upload(&buf, data)?;
        Ok(buf)
    }

    /// Copy `data` from CPU (host) into an existing device buffer.
    pub fn upload(&self, buf: &CudaBuffer, data: &[u8]) -> Result<(), CudaError> {
        if data.len() > buf.len {
            return Err(CudaError::DriverError {
                code: -1,
                message: format!(
                    "upload: data ({} bytes) exceeds buffer capacity ({} bytes)",
                    data.len(), buf.len
                ),
            });
        }
        self.bind_ctx()?;
        unsafe {
            self.cuda.check((self.cuda.cu_memcpy_h_to_d)(
                buf.dptr,
                data.as_ptr() as *const c_void,
                data.len(),
            ))
        }
    }

    /// Copy device buffer contents to a CPU `Vec<u8>`.
    pub fn download(&self, buf: &CudaBuffer) -> Result<Vec<u8>, CudaError> {
        self.bind_ctx()?;
        let mut out = vec![0u8; buf.len];
        unsafe {
            self.cuda.check((self.cuda.cu_memcpy_d_to_h)(
                out.as_mut_ptr() as *mut c_void,
                buf.dptr,
                buf.len,
            ))?;
        }
        Ok(out)
    }

    /// Compile CUDA C source to a `CudaModule`.
    ///
    /// Uses NVRTC to compile the source to PTX, then loads the PTX into
    /// the Driver API as a `CUmodule`.
    pub fn compile(&self, source: &str) -> Result<CudaModule, CudaError> {
        self.bind_ctx()?;
        let ptx = self.nvrtc.compile(source)?;
        let c_ptx = CString::new(ptx).map_err(|_| {
            CudaError::CompileFailed("PTX contains internal NUL bytes".to_string())
        })?;
        unsafe {
            let mut module: CUmodule = std::ptr::null_mut();
            self.cuda.check((self.cuda.cu_module_load_data)(
                &mut module,
                c_ptx.as_ptr() as *const c_void,
            ))?;
            Ok(CudaModule {
                inner: std::sync::Arc::new(CudaModuleInner {
                    cuda: self.cuda.clone(),
                    module,
                }),
            })
        }
    }

    /// Launch a kernel.
    ///
    /// `grid`   — threadblock grid dimensions (x, y, z).
    /// `block`  — threads per block (x, y, z).
    /// `args`   — raw pointers to kernel argument values on the CPU.
    ///   Each element is a `*mut *mut c_void` pointing to the argument.
    ///
    /// Use [`CudaBuffer::as_kernel_arg`] for device pointers and pass
    /// scalar args as `&value as *const T as *mut c_void`.
    pub fn launch(
        &self,
        func:  &CudaFunction,
        grid:  [u32; 3],
        block: [u32; 3],
        args:  &mut [*mut c_void],
    ) -> Result<(), CudaError> {
        if grid.iter().any(|&d| d == 0) || block.iter().any(|&d| d == 0) {
            return Err(CudaError::DriverError {
                code: -1,
                message: format!(
                    "launch: all grid/block dimensions must be >= 1 (grid={grid:?}, block={block:?})"
                ),
            });
        }
        self.bind_ctx()?;
        unsafe {
            self.cuda.check((self.cuda.cu_launch_kernel)(
                func.function,
                grid[0], grid[1], grid[2],
                block[0], block[1], block[2],
                0,                    // shared memory bytes (dynamic)
                CU_STREAM_LEGACY,     // default stream
                args.as_mut_ptr(),
                std::ptr::null_mut(), // extra (not used)
            ))
        }
    }

    /// Wait for all in-flight GPU operations to complete.
    ///
    /// Always call this after `launch()` before reading results.
    pub fn synchronize(&self) -> Result<(), CudaError> {
        self.bind_ctx()?;
        unsafe { self.cuda.check((self.cuda.cu_ctx_synchronize)()) }
    }
}

// --------------------------------------------------------------------------
// CudaBuffer
// --------------------------------------------------------------------------

/// A region of GPU device memory.
///
/// Created by `CudaDevice::alloc()`.  Freed when dropped.
pub struct CudaBuffer {
    cuda: std::sync::Arc<CudaLib>,
    dptr: CUdeviceptr,
    len:  usize,
}

impl Drop for CudaBuffer {
    fn drop(&mut self) {
        if self.dptr != 0 {
            unsafe { (self.cuda.cu_mem_free)(self.dptr) };
        }
    }
}

impl CudaBuffer {
    pub fn len(&self) -> usize {
        self.len
    }

    /// The raw CUDA device pointer value.
    ///
    /// Pass this (via a `*mut CUdeviceptr` pointer) to `cuLaunchKernel`
    /// kernel params so the kernel receives the actual GPU memory address.
    pub fn device_ptr(&self) -> CUdeviceptr {
        self.dptr
    }

    /// Return this buffer's device pointer as a kernel argument pointer.
    ///
    /// CUDA `cuLaunchKernel` takes `kernel_params` as an array of
    /// `*mut *mut c_void` — one pointer-to-pointer per argument.
    /// This method returns a mutable pointer to the internal `CUdeviceptr`
    /// field, cast appropriately.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid for the lifetime of `self` AND only
    /// while `self` has not been moved since this call.  Moving `CudaBuffer`
    /// (e.g. into a closure or another variable) invalidates the pointer
    /// because the `CUdeviceptr` field changes address.  The caller must
    /// ensure the pointer is used before any such move occurs — typically by
    /// assembling the args array and calling `launch` in the same expression.
    /// Prefer `device_ptr()` + manual args construction for clarity.
    pub unsafe fn as_kernel_arg(&mut self) -> *mut c_void {
        &mut self.dptr as *mut u64 as *mut c_void
    }
}

// --------------------------------------------------------------------------
// CudaModule
// --------------------------------------------------------------------------

// Inner state shared between CudaModule and every CudaFunction derived from
// it.  Wrapping in Arc means the PTX module is not unloaded (cuModuleUnload)
// until both the CudaModule AND all derived CudaFunctions are dropped —
// eliminating the use-after-free that would occur if the module is dropped
// while a CudaFunction is still alive.
struct CudaModuleInner {
    cuda:   std::sync::Arc<CudaLib>,
    module: CUmodule,
}

impl Drop for CudaModuleInner {
    fn drop(&mut self) {
        if !self.module.is_null() {
            unsafe { (self.cuda.cu_module_unload)(self.module) };
        }
    }
}

// CUmodule is an opaque driver handle; CUDA docs say it is safe to transfer
// between threads (the context, not the module, is the thread-bound entity).
unsafe impl Send for CudaModuleInner {}

/// A loaded CUDA module (compiled PTX).
///
/// Contains one or more `__global__` kernel functions.
pub struct CudaModule {
    inner: std::sync::Arc<CudaModuleInner>,
}

impl CudaModule {
    /// Get a kernel function by name.
    ///
    /// The `name` must match a `__global__` function in the compiled source
    /// with C linkage (`extern "C"`).
    ///
    /// The returned `CudaFunction` holds an internal reference to this module,
    /// so the PTX module remains loaded until both the `CudaModule` and the
    /// `CudaFunction` are dropped.
    pub fn function(&self, name: &str) -> Result<CudaFunction, CudaError> {
        let c_name = CString::new(name)
            .map_err(|_| CudaError::FunctionNotFound(name.to_string()))?;
        let mut func: CUfunction = std::ptr::null_mut();
        unsafe {
            self.inner.cuda.check((self.inner.cuda.cu_module_get_function)(
                &mut func,
                self.inner.module,
                c_name.as_ptr(),
            ))?;
        }
        if func.is_null() {
            return Err(CudaError::FunctionNotFound(name.to_string()));
        }
        Ok(CudaFunction {
            function: func,
            _module: self.inner.clone(),
        })
    }
}

// --------------------------------------------------------------------------
// CudaFunction
// --------------------------------------------------------------------------

/// A `__global__` kernel function, ready to launch.
///
/// Obtained from `CudaModule::function()`.  Holds an `Arc` to the parent
/// module's inner state, so the PTX module stays loaded for the entire
/// lifetime of this handle.
pub struct CudaFunction {
    pub(crate) function: CUfunction,
    _module: std::sync::Arc<CudaModuleInner>,
}

// CUfunction is an opaque driver handle; safe to transfer between threads.
unsafe impl Send for CudaFunction {}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Check that `CudaDevice::new()` returns a meaningful error on machines
    /// without CUDA installed (the common case in CI and on Mac).
    #[test]
    fn no_cuda_returns_not_available_or_nvrtc_not_found() {
        let result = CudaDevice::new(0);
        match result {
            Ok(_) => {
                // CUDA is available — skip further assertions.
                eprintln!("CUDA device found; tests running with real GPU.");
            }
            Err(CudaError::NotAvailable) => {
                eprintln!("CUDA not available (expected on Mac/no-GPU CI).");
            }
            Err(CudaError::NvrtcNotFound) => {
                eprintln!("CUDA driver found but no NVRTC toolkit.");
            }
            Err(e) => {
                panic!("unexpected error: {e}");
            }
        }
    }

    #[test]
    fn device_count_zero_or_more() {
        let result = CudaDevice::device_count();
        // On non-CUDA machines, NotAvailable; on CUDA machines, >= 1.
        match result {
            Ok(n) => assert!(n >= 0, "device count must be non-negative"),
            Err(CudaError::NotAvailable) => {} // expected on Mac/CI
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    /// Full round-trip test — only runs when CUDA is actually present.
    #[test]
    fn invert_kernel_round_trip() {
        let device = match CudaDevice::new(0) {
            Ok(d)  => d,
            Err(_) => return, // skip if no CUDA
        };

        const CUDA_C: &str = r#"
        extern "C" __global__ void invert(
            const unsigned char* src,
            unsigned char* dst,
            unsigned int n
        ) {
            unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
            if (gid >= n) return;
            dst[gid] = 255 - src[gid];
        }
        "#;

        let src_data: Vec<u8> = (0u8..=255).collect();
        let expected: Vec<u8> = src_data.iter().map(|&b| 255 - b).collect();
        let n = src_data.len() as u32;

        let src_buf = device.alloc_with_bytes(&src_data).unwrap();
        let mut dst_buf = device.alloc(src_data.len()).unwrap();

        let module   = device.compile(CUDA_C).unwrap();
        let function = module.function("invert").unwrap();

        let mut src_arg = src_buf.dptr;
        let mut dst_arg = dst_buf.dptr;
        let mut n_arg   = n;

        let mut args: [*mut c_void; 3] = [
            &mut src_arg as *mut _ as *mut c_void,
            &mut dst_arg as *mut _ as *mut c_void,
            &mut n_arg   as *mut _ as *mut c_void,
        ];

        device.launch(&function, [1, 1, 1], [256, 1, 1], &mut args).unwrap();
        device.synchronize().unwrap();

        let result = device.download(&dst_buf).unwrap();
        assert_eq!(result, expected);
    }
}
