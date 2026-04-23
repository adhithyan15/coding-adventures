//! # metal-compute — Real Metal GPU Compute on macOS
//!
//! G09 Layer 4B: This crate wraps Apple's Metal compute API (via
//! `objc-bridge`) into clean Rust types.  It is **macOS-only**; on other
//! platforms every public constructor returns `Err(MetalError::NotSupported)`.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use metal_compute::{MetalDevice, MetalError};
//!
//! // MSL compute kernel — one thread per byte, inverts it.
//! const MSL: &str = r#"
//! #include <metal_stdlib>
//! using namespace metal;
//! kernel void invert(
//!     device const uint8_t* src [[buffer(0)]],
//!     device       uint8_t* dst [[buffer(1)]],
//!     constant     uint&    n   [[buffer(2)]],
//!     uint gid [[thread_position_in_grid]]
//! ) {
//!     if (gid >= n) return;
//!     dst[gid] = 255 - src[gid];
//! }
//! "#;
//!
//! fn invert(pixels: &[u8]) -> Result<Vec<u8>, MetalError> {
//!     let device = MetalDevice::new()?;
//!     let queue  = device.command_queue();
//!
//!     let src_buf = device.alloc_with_bytes(pixels);
//!     let dst_buf = device.alloc(pixels.len());
//!     let n_bytes: u32 = pixels.len() as u32;
//!
//!     let lib      = device.compile(MSL)?;
//!     let func     = lib.function("invert")?;
//!     let pipeline = device.pipeline(&func)?;
//!
//!     queue.dispatch(|enc| {
//!         enc.set_pipeline(&pipeline);
//!         enc.set_buffer(&src_buf, 0);
//!         enc.set_buffer(&dst_buf, 1);
//!         enc.set_bytes(n_bytes.to_le_bytes().as_ref(), 2);
//!         // One thread per byte.  Metal picks the group size from the pipeline.
//!         enc.dispatch_threads(pixels.len() as u32);
//!     });
//!
//!     Ok(dst_buf.to_vec())
//! }
//! ```
//!
//! ## Memory model: unified memory on Apple Silicon
//!
//! On Apple M-series chips (M1, M2, M3, …) the CPU and GPU share one
//! physical memory die.  A `MetalBuffer` allocated with `Shared` storage
//! mode is a pointer into this unified pool.  Both sides see writes
//! immediately — there is no PCIe bus transfer and no explicit "upload"
//! or "download" step.
//!
//! ```text
//! CPU side          Shared DRAM            GPU side
//! buf.as_slice() ──→ [0x…] ←── kernel reads via [[buffer(N)]]
//! ```
//!
//! After `queue.dispatch()` returns, the CPU can read the output buffer
//! directly.  `to_vec()` copies the bytes out; or, for chained operations,
//! pass the buffer directly to the next dispatch.
//!
//! ## Metal dispatch model
//!
//! Metal's compute model divides work into a two-level grid:
//!
//! ```text
//! Total grid = threadgroups × threads_per_threadgroup
//!
//! e.g. image 1920×1080, threadgroup 8×8:
//!   threadgroups  = (⌈1920/8⌉, ⌈1080/8⌉) = (240, 135)
//!   threads/group = (8, 8)
//!   Total threads = 240*8 × 135*8 = 1920 × 1080 ✓
//! ```
//!
//! Each thread sees its position via `gid = [[thread_position_in_grid]]`
//! (1D, 2D, or 3D).  Guard-returns are used for excess threads at edges.

pub const VERSION: &str = "0.1.0";

// --------------------------------------------------------------------------
// Error type
// --------------------------------------------------------------------------

/// Errors that `metal-compute` can return.
#[derive(Debug, Clone)]
pub enum MetalError {
    /// Metal is not available on this platform (non-Apple builds).
    NotSupported,
    /// `MTLCreateSystemDefaultDevice()` returned nil — unlikely on modern Mac.
    NoDevice,
    /// MSL source compilation failed.  The message comes from the Metal driver.
    CompileFailed(String),
    /// A named `kernel` function was not found in the compiled library.
    FunctionNotFound(String),
    /// `newComputePipelineStateWithFunction:error:` failed.
    PipelineFailed(String),
    /// `newBufferWithLength:options:` returned nil — allocation failed.
    AllocationFailed(usize),
}

impl std::fmt::Display for MetalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetalError::NotSupported        => write!(f, "Metal is not supported on this platform"),
            MetalError::NoDevice            => write!(f, "MTLCreateSystemDefaultDevice returned nil"),
            MetalError::CompileFailed(m)    => write!(f, "MSL compilation failed: {m}"),
            MetalError::FunctionNotFound(n) => write!(f, "kernel function '{n}' not found in library"),
            MetalError::PipelineFailed(m)   => write!(f, "compute pipeline creation failed: {m}"),
            MetalError::AllocationFailed(n) => write!(f, "MTLBuffer allocation failed for {n} bytes"),
        }
    }
}

impl std::error::Error for MetalError {}

// --------------------------------------------------------------------------
// Apple-only implementation
// --------------------------------------------------------------------------

#[cfg(target_vendor = "apple")]
mod apple {
    use super::MetalError;
    use objc_bridge::{
        Id, MTLSize, MTL_RESOURCE_OPTIONS_DEFAULT, NIL,
        MTLCreateSystemDefaultDevice,
        msg, msg_ptr, msg_u64,
        cfstring, sel, release, retain,
        CFRelease,
    };
    use std::ffi::c_void;

    // RAII guard that calls `release()` on drop.  Used in `dispatch()` to
    // guarantee cleanup even when the user closure panics.
    struct AutoRelease(Id);
    impl Drop for AutoRelease {
        fn drop(&mut self) {
            unsafe { release(self.0) };
        }
    }

    // ------------------------------------------------------------------
    // MetalDevice
    // ------------------------------------------------------------------

    /// A connection to one Metal GPU.
    ///
    /// Wraps `id<MTLDevice>`.  The underlying Objective-C object is
    /// retained on construction and released when `MetalDevice` is dropped.
    pub struct MetalDevice {
        pub(crate) device: Id,
    }

    impl Drop for MetalDevice {
        fn drop(&mut self) {
            unsafe { release(self.device) };
        }
    }

    impl MetalDevice {
        /// Open a connection to the system's default GPU.
        ///
        /// Calls `MTLCreateSystemDefaultDevice()` which returns the GPU
        /// that handles the main display.  On Apple Silicon this is always
        /// the integrated GPU (which IS the only GPU).
        ///
        /// Returns `Err(MetalError::NoDevice)` if no Metal-capable GPU is
        /// present (extremely rare — all Macs since 2012 support Metal).
        pub fn new() -> Result<Self, MetalError> {
            unsafe {
                let device = MTLCreateSystemDefaultDevice();
                if device == NIL {
                    return Err(MetalError::NoDevice);
                }
                Ok(MetalDevice { device })
            }
        }

        /// Allocate a shared-memory buffer of `len` bytes.
        ///
        /// Uses `MTLResourceStorageModeShared` so the same physical bytes
        /// are accessible from both CPU (`as_slice`) and GPU (`[[buffer(N)]]`
        /// in MSL).  No transfer cost on Apple Silicon.
        pub fn alloc(&self, len: usize) -> Result<MetalBuffer, MetalError> {
            unsafe {
                // [device newBufferWithLength:len options:MTL_RESOURCE_OPTIONS_DEFAULT]
                let f: unsafe extern "C" fn(Id, objc_bridge::Sel, usize, usize) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                let buf = f(
                    self.device,
                    sel("newBufferWithLength:options:"),
                    len,
                    MTL_RESOURCE_OPTIONS_DEFAULT as usize,
                );
                if buf.is_null() {
                    return Err(MetalError::AllocationFailed(len));
                }
                Ok(MetalBuffer { buffer: buf, len })
            }
        }

        /// Allocate a buffer and immediately copy `data` into it.
        ///
        /// Equivalent to `alloc(data.len())` followed by writing to `as_slice_mut()`,
        /// but uses `newBufferWithBytes:length:options:` directly for efficiency.
        pub fn alloc_with_bytes(&self, data: &[u8]) -> Result<MetalBuffer, MetalError> {
            unsafe {
                let f: unsafe extern "C" fn(Id, objc_bridge::Sel, *const c_void, usize, usize) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                let buf = f(
                    self.device,
                    sel("newBufferWithBytes:length:options:"),
                    data.as_ptr() as *const c_void,
                    data.len(),
                    MTL_RESOURCE_OPTIONS_DEFAULT as usize,
                );
                if buf.is_null() {
                    return Err(MetalError::AllocationFailed(data.len()));
                }
                Ok(MetalBuffer { buffer: buf, len: data.len() })
            }
        }

        /// Compile MSL source code and return a `MetalLibrary`.
        ///
        /// The source string may contain any number of `kernel` functions.
        /// Compilation is done by the Metal driver at runtime — no offline
        /// toolchain is required.
        ///
        /// # Errors
        ///
        /// Returns `MetalError::CompileFailed` with the driver's error message
        /// if the source contains a syntax or type error.
        pub fn compile(&self, source: &str) -> Result<MetalLibrary, MetalError> {
            unsafe {
                // [device newLibraryWithSource:source options:nil error:&err]
                let ns_source = cfstring(source);
                let mut error: Id = NIL;

                let f: unsafe extern "C" fn(Id, objc_bridge::Sel, Id, Id, *mut Id) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                let library = f(
                    self.device,
                    sel("newLibraryWithSource:options:error:"),
                    ns_source,
                    NIL,
                    &mut error as *mut Id,
                );
                CFRelease(ns_source);

                if library == NIL {
                    let msg = if error != NIL {
                        extract_ns_string(msg!(error, "localizedDescription"))
                    } else {
                        "unknown error".to_string()
                    };
                    return Err(MetalError::CompileFailed(msg));
                }

                Ok(MetalLibrary { library })
            }
        }

        /// Create a compute pipeline for the given `MetalFunction`.
        ///
        /// A pipeline state object is the compiled, GPU-ready form of a
        /// kernel function.  Creating it is expensive; cache the result
        /// and reuse across dispatches.
        pub fn pipeline(
            &self,
            function: &MetalFunction,
        ) -> Result<MetalComputePipeline, MetalError> {
            unsafe {
                let mut error: Id = NIL;

                let f: unsafe extern "C" fn(Id, objc_bridge::Sel, Id, *mut Id) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                let pso = f(
                    self.device,
                    sel("newComputePipelineStateWithFunction:error:"),
                    function.function,
                    &mut error as *mut Id,
                );

                if pso == NIL {
                    let msg = if error != NIL {
                        extract_ns_string(msg!(error, "localizedDescription"))
                    } else {
                        "unknown error".to_string()
                    };
                    return Err(MetalError::PipelineFailed(msg));
                }

                // Ask the pipeline what threadgroup size it prefers.
                // On Apple Silicon, this is the thread-execution width
                // (e.g. 32 for M1) for 1D work, or a 2D tile for 2D work.
                let max_threads = msg_u64!(pso, "maxTotalThreadsPerThreadgroup");
                let thread_width = msg_u64!(pso, "threadExecutionWidth");

                Ok(MetalComputePipeline { pso, max_threads, thread_width })
            }
        }

        /// Create a command queue.
        ///
        /// A `MetalCommandQueue` is a channel to the GPU.  One queue is
        /// enough for serial workloads.  Create multiple queues for
        /// concurrent overlapping dispatches.
        pub fn command_queue(&self) -> MetalCommandQueue {
            unsafe {
                let queue = msg!(self.device, "newCommandQueue");
                assert!(!queue.is_null(), "MetalDevice::command_queue: newCommandQueue returned nil");
                MetalCommandQueue { queue }
            }
        }
    }

    // ------------------------------------------------------------------
    // MetalBuffer
    // ------------------------------------------------------------------

    /// A contiguous region of memory accessible to both CPU and GPU.
    ///
    /// On Apple Silicon (unified memory architecture), this is physically
    /// the same RAM that the CPU sees.  Writes on either side are
    /// immediately visible to the other — there is no transfer step.
    ///
    /// Wraps `id<MTLBuffer>`.  Released when dropped.
    pub struct MetalBuffer {
        pub(crate) buffer: Id,
        len: usize,
    }

    impl Drop for MetalBuffer {
        fn drop(&mut self) {
            unsafe { release(self.buffer) };
        }
    }

    impl MetalBuffer {
        /// Byte length of the buffer.
        pub fn len(&self) -> usize {
            self.len
        }

        /// Mutable byte view into the CPU-side of the buffer.
        ///
        /// On Apple Silicon, the GPU also sees these bytes via `[[buffer(N)]]`.
        /// Do NOT write here while a GPU dispatch is in flight.
        ///
        /// # Safety
        ///
        /// The caller must ensure no concurrent GPU access during the lifetime
        /// of the mutable reference.
        pub fn as_slice_mut(&mut self) -> &mut [u8] {
            unsafe {
                let ptr = msg_ptr!(self.buffer, "contents") as *mut u8;
                assert!(!ptr.is_null(), "MetalBuffer::as_slice_mut: contents() returned null");
                std::slice::from_raw_parts_mut(ptr, self.len)
            }
        }

        /// Read-only byte view into the CPU-side of the buffer.
        pub fn as_slice(&self) -> &[u8] {
            unsafe {
                let ptr = msg_ptr!(self.buffer, "contents") as *const u8;
                assert!(!ptr.is_null(), "MetalBuffer::as_slice: contents() returned null");
                std::slice::from_raw_parts(ptr, self.len)
            }
        }

        /// Copy the buffer contents into a new `Vec<u8>`.
        pub fn to_vec(&self) -> Vec<u8> {
            self.as_slice().to_vec()
        }
    }

    // ------------------------------------------------------------------
    // MetalLibrary
    // ------------------------------------------------------------------

    /// A compiled MSL shader library.
    ///
    /// Wraps `id<MTLLibrary>`.  Contains one or more `kernel` functions
    /// that can be retrieved by name.
    pub struct MetalLibrary {
        library: Id,
    }

    impl Drop for MetalLibrary {
        fn drop(&mut self) {
            unsafe { release(self.library) };
        }
    }

    impl MetalLibrary {
        /// Look up a `kernel` function by name.
        ///
        /// Returns `Err(MetalError::FunctionNotFound)` if the library does
        /// not contain a function with that name.
        pub fn function(&self, name: &str) -> Result<MetalFunction, MetalError> {
            unsafe {
                let ns_name = cfstring(name);
                let func = msg!(self.library, "newFunctionWithName:", ns_name);
                CFRelease(ns_name);

                if func == NIL {
                    return Err(MetalError::FunctionNotFound(name.to_string()));
                }
                Ok(MetalFunction { function: func })
            }
        }
    }

    // ------------------------------------------------------------------
    // MetalFunction
    // ------------------------------------------------------------------

    /// A single `kernel` function inside a compiled `MetalLibrary`.
    ///
    /// Wraps `id<MTLFunction>`.  Pass to `MetalDevice::pipeline()` to
    /// create a ready-to-dispatch `MetalComputePipeline`.
    pub struct MetalFunction {
        pub(crate) function: Id,
    }

    impl Drop for MetalFunction {
        fn drop(&mut self) {
            unsafe { release(self.function) };
        }
    }

    // ------------------------------------------------------------------
    // MetalComputePipeline
    // ------------------------------------------------------------------

    /// A compiled compute pipeline state.
    ///
    /// Wraps `id<MTLComputePipelineState>`.  Create once, reuse across
    /// many dispatches.  This is the compiled, GPU-resident form of a
    /// kernel function.
    ///
    /// `max_threads` and `thread_width` are queried from the pipeline and
    /// exposed so callers can choose an efficient threadgroup size.
    pub struct MetalComputePipeline {
        pub(crate) pso: Id,
        /// Maximum threads per threadgroup on this GPU.
        pub max_threads: u64,
        /// The GPU's SIMD width (e.g. 32 on M1).  Threadgroup sizes that
        /// are multiples of this fill SIMD lanes efficiently.
        pub thread_width: u64,
    }

    impl Drop for MetalComputePipeline {
        fn drop(&mut self) {
            unsafe { release(self.pso) };
        }
    }

    impl MetalComputePipeline {
        /// Compute an efficient 1D threadgroup size for this pipeline.
        ///
        /// Returns the largest power-of-two that fits in `max_threads` and
        /// is a multiple of `thread_width`.  Typically 256 or 512.
        pub fn preferred_threads_1d(&self) -> u32 {
            let mut size = self.thread_width.min(self.max_threads);
            while size * 2 <= self.max_threads && size * 2 % self.thread_width == 0 {
                size *= 2;
            }
            size as u32
        }

        /// Compute an efficient 2D threadgroup size (width, height).
        ///
        /// Returns (w, h) such that w * h ≤ `max_threads` and both
        /// dimensions are multiples of `thread_width` where possible.
        /// Typically (16, 16) → 256 threads on M1.
        pub fn preferred_threads_2d(&self) -> (u32, u32) {
            let side = (self.max_threads as f64).sqrt() as u64;
            // Round down to nearest multiple of thread_width.
            let side = (side / self.thread_width.max(1)) * self.thread_width.max(1);
            let side = side.max(1) as u32;
            (side, side)
        }
    }

    // ------------------------------------------------------------------
    // MetalCommandQueue
    // ------------------------------------------------------------------

    /// A channel for submitting work to the GPU.
    ///
    /// Wraps `id<MTLCommandQueue>`.  Call `dispatch()` to record and
    /// submit one compute pass.  Multiple dispatches on the same queue
    /// are serialised (no overlap); create multiple queues for pipelining.
    pub struct MetalCommandQueue {
        queue: Id,
    }

    impl Drop for MetalCommandQueue {
        fn drop(&mut self) {
            unsafe { release(self.queue) };
        }
    }

    impl MetalCommandQueue {
        /// Record a compute pass and submit it to the GPU.  Blocks until
        /// the GPU finishes all commands in the pass.
        ///
        /// The closure receives a `&mut MetalComputeEncoder` for recording
        /// commands.  After the closure returns, the encoder is ended, the
        /// command buffer is committed, and this function waits for completion.
        ///
        /// On Apple Silicon this typically returns in microseconds to
        /// milliseconds depending on kernel complexity and data size.
        pub fn dispatch<F: FnOnce(&mut MetalComputeEncoder)>(&self, f: F) {
            unsafe {
                // `commandBuffer` and `computeCommandEncoder` return autoreleased
                // objects (no `new` prefix).  Retain them explicitly so they
                // stay alive through commit + waitUntilCompleted regardless of
                // whether an NSAutoreleasePool is in scope.
                //
                // AutoRelease guards ensure release() is called even if `f` panics.
                // Rust drops in reverse declaration order: encoder first, then
                // cmd_buf — matching the expected teardown sequence.
                let cmd_buf = retain(msg!(self.queue, "commandBuffer"));
                assert!(!cmd_buf.is_null(), "commandBuffer returned nil");
                let _cmd_guard = AutoRelease(cmd_buf);

                let encoder_obj = retain(msg!(cmd_buf, "computeCommandEncoder"));
                assert!(!encoder_obj.is_null(), "computeCommandEncoder returned nil");
                let _enc_guard = AutoRelease(encoder_obj);

                let mut encoder = MetalComputeEncoder { encoder: encoder_obj };
                f(&mut encoder);

                msg!(encoder_obj, "endEncoding");
                msg!(cmd_buf, "commit");
                msg!(cmd_buf, "waitUntilCompleted");
                // _enc_guard drops → release(encoder_obj)
                // _cmd_guard drops → release(cmd_buf)
            }
        }
    }

    // ------------------------------------------------------------------
    // MetalComputeEncoder
    // ------------------------------------------------------------------

    /// Records compute commands into a command buffer.
    ///
    /// Created inside a `queue.dispatch(|enc| { … })` closure.  Commands
    /// are issued in order; the encoder is ended automatically when the
    /// closure returns.
    pub struct MetalComputeEncoder {
        pub(crate) encoder: Id,
    }

    impl MetalComputeEncoder {
        /// Set the pipeline for all subsequent `dispatch_*` calls.
        ///
        /// Must be called once before any dispatch call.
        pub fn set_pipeline(&mut self, pipeline: &MetalComputePipeline) {
            unsafe {
                let f: unsafe extern "C" fn(Id, objc_bridge::Sel, Id) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                f(
                    self.encoder,
                    sel("setComputePipelineState:"),
                    pipeline.pso,
                );
            }
        }

        /// Bind a `MetalBuffer` at `index`.
        ///
        /// The index must match the `[[buffer(N)]]` attribute on the MSL
        /// kernel parameter.  Offset is 0 (the full buffer is visible).
        pub fn set_buffer(&mut self, buffer: &MetalBuffer, index: u64) {
            unsafe {
                let f: unsafe extern "C" fn(Id, objc_bridge::Sel, Id, usize, u64) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                f(
                    self.encoder,
                    sel("setBuffer:offset:atIndex:"),
                    buffer.buffer,
                    0usize,
                    index,
                );
            }
        }

        /// Write a small inline value directly into the encoder (no buffer allocation).
        ///
        /// Use for uniforms (width, height, sigma, …).  The `data` slice
        /// must match the layout expected by the `constant T&` MSL parameter
        /// at the given `index`.
        ///
        /// Maximum size: 4 KB (Metal limit for inline data).
        pub fn set_bytes(&mut self, data: &[u8], index: u64) {
            unsafe {
                let f: unsafe extern "C" fn(
                    Id, objc_bridge::Sel, *const c_void, usize, u64
                ) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                f(
                    self.encoder,
                    sel("setBytes:length:atIndex:"),
                    data.as_ptr() as *const c_void,
                    data.len(),
                    index,
                );
            }
        }

        /// Dispatch threads in a 1D grid.
        ///
        /// `total_threads` — total number of invocations (often image width×height).
        /// `threads_per_group` — how many threads share a threadgroup;
        /// must be ≤ `pipeline.max_threads`.
        ///
        /// Metal rounds up the last threadgroup automatically — no need to
        /// pad `total_threads` to a multiple of `threads_per_group`.
        pub fn dispatch_threads_1d(&mut self, total_threads: u32, threads_per_group: u32) {
            unsafe {
                // [encoder dispatchThreads:MTLSizeMake(total,1,1)
                //           threadsPerThreadgroup:MTLSizeMake(tpg,1,1)]
                let f: unsafe extern "C" fn(
                    Id, objc_bridge::Sel, MTLSize, MTLSize
                ) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                f(
                    self.encoder,
                    sel("dispatchThreads:threadsPerThreadgroup:"),
                    MTLSize { width: total_threads as _, height: 1, depth: 1 },
                    MTLSize { width: threads_per_group as _, height: 1, depth: 1 },
                );
            }
        }

        /// Dispatch threads in a 2D grid.
        ///
        /// Use this for image kernels where each thread processes one pixel.
        ///
        /// `(width, height)` — image dimensions; threads are dispatched for
        /// every (x, y) in [0, width) × [0, height).
        /// `(tpg_w, tpg_h)` — threadgroup size; typically (8,8), (16,16), or
        /// whatever `pipeline.preferred_threads_2d()` returns.
        ///
        /// The kernel should guard with:
        /// ```metal
        /// uint2 gid [[thread_position_in_grid]];
        /// if (gid.x >= width || gid.y >= height) return;
        /// ```
        pub fn dispatch_threads_2d(
            &mut self,
            width: u32, height: u32,
            tpg_w: u32, tpg_h: u32,
        ) {
            unsafe {
                let f: unsafe extern "C" fn(
                    Id, objc_bridge::Sel, MTLSize, MTLSize
                ) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                f(
                    self.encoder,
                    sel("dispatchThreads:threadsPerThreadgroup:"),
                    MTLSize { width: width as _, height: height as _, depth: 1 },
                    MTLSize { width: tpg_w as _, height: tpg_h as _, depth: 1 },
                );
            }
        }

        /// Low-level: dispatch threadgroups explicitly (original Metal API).
        ///
        /// Use `dispatch_threads_2d` instead unless you need exact threadgroup
        /// control.  Metal 3 and later support `dispatchThreads:` which
        /// handles non-multiple dimensions automatically; this uses
        /// `dispatchThreadgroups:threadsPerThreadgroup:` which is universal
        /// but requires the caller to compute `ceil(dim / tpg_size)`.
        pub fn dispatch_threadgroups(
            &mut self,
            threadgroups: MTLSize,
            threads_per_group: MTLSize,
        ) {
            unsafe {
                let f: unsafe extern "C" fn(
                    Id, objc_bridge::Sel, MTLSize, MTLSize
                ) -> Id =
                    ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
                f(
                    self.encoder,
                    sel("dispatchThreadgroups:threadsPerThreadgroup:"),
                    threadgroups,
                    threads_per_group,
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // Thread-safety impls
    // ------------------------------------------------------------------
    //
    // Apple's Metal documentation explicitly states: "Metal objects are
    // generally thread-safe. Your app can create a Metal object on one
    // thread and send it to another thread."
    //
    // Raw pointer fields (`*mut Object`) are !Send + !Sync by default,
    // but the Metal runtime guarantees Send safety — we assert it here.
    // Sync is intentionally NOT implemented for MetalCommandQueue and
    // MetalComputePipeline; callers that need shared access should wrap
    // them in a Mutex (Mutex<T>: Sync requires only T: Send, not T: Sync).

    unsafe impl Send for MetalDevice {}
    unsafe impl Sync for MetalDevice {}

    unsafe impl Send for MetalCommandQueue {}
    // MetalCommandQueue: not Sync — wrap in Mutex for shared access.

    // MetalBuffer: Send (can be moved to another thread), but NOT Sync
    // (concurrent mutable access via as_slice_mut() would be a data race).
    unsafe impl Send for MetalBuffer {}

    unsafe impl Send for MetalLibrary {}
    unsafe impl Sync for MetalLibrary {}

    unsafe impl Send for MetalFunction {}
    unsafe impl Send for MetalComputePipeline {}
    // MetalComputePipeline: not Sync — wrap in Mutex for shared access.

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Read an NSString / CFString into a Rust String.
    ///
    /// Calls `[nsstring UTF8String]` which returns a borrowed C string;
    /// we copy it into owned Rust memory.  The NSString must be valid.
    unsafe fn extract_ns_string(ns_str: Id) -> String {
        if ns_str.is_null() {
            return "<nil NSString>".to_string();
        }
        let f: unsafe extern "C" fn(Id, objc_bridge::Sel) -> *const std::ffi::c_char =
            ::std::mem::transmute(objc_bridge::objc_msgSend as *const ());
        let c_ptr = f(ns_str, sel("UTF8String"));
        if c_ptr.is_null() {
            return "<null UTF8String>".to_string();
        }
        std::ffi::CStr::from_ptr(c_ptr)
            .to_string_lossy()
            .into_owned()
    }
}

// --------------------------------------------------------------------------
// Public re-exports (Apple)
// --------------------------------------------------------------------------

#[cfg(target_vendor = "apple")]
pub use apple::{
    MetalBuffer, MetalCommandQueue, MetalComputeEncoder, MetalComputePipeline,
    MetalDevice, MetalFunction, MetalLibrary,
};

// --------------------------------------------------------------------------
// Stub types for non-Apple platforms (for cross-platform Cargo builds)
// --------------------------------------------------------------------------

/// `MetalDevice` on non-Apple platforms — always errors.
#[cfg(not(target_vendor = "apple"))]
pub struct MetalDevice;

#[cfg(not(target_vendor = "apple"))]
impl MetalDevice {
    pub fn new() -> Result<Self, MetalError> {
        Err(MetalError::NotSupported)
    }
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(all(test, target_vendor = "apple"))]
mod tests {
    use super::apple::*;
    use super::MetalError;

    /// A minimal MSL kernel for testing: copies each byte unchanged.
    const MSL_IDENTITY: &str = r#"
        #include <metal_stdlib>
        using namespace metal;
        kernel void identity(
            device const uint8_t* src [[buffer(0)]],
            device       uint8_t* dst [[buffer(1)]],
            constant     uint&    n   [[buffer(2)]],
            uint gid [[thread_position_in_grid]]
        ) {
            if (gid >= n) return;
            dst[gid] = src[gid];
        }
    "#;

    /// Invert all bytes: dst[i] = 255 - src[i].
    const MSL_INVERT: &str = r#"
        #include <metal_stdlib>
        using namespace metal;
        kernel void invert(
            device const uint8_t* src [[buffer(0)]],
            device       uint8_t* dst [[buffer(1)]],
            constant     uint&    n   [[buffer(2)]],
            uint gid [[thread_position_in_grid]]
        ) {
            if (gid >= n) return;
            dst[gid] = 255 - src[gid];
        }
    "#;

    #[test]
    fn can_create_device() {
        let d = MetalDevice::new();
        assert!(d.is_ok(), "expected Metal device, got {:?}", d.err());
    }

    #[test]
    fn can_compile_msl() {
        let d = MetalDevice::new().unwrap();
        let lib = d.compile(MSL_IDENTITY);
        assert!(lib.is_ok(), "MSL compile failed: {:?}", lib.err());
    }

    #[test]
    fn compile_error_is_reported() {
        let d = MetalDevice::new().unwrap();
        let result = d.compile("this is not valid MSL;");
        assert!(matches!(result, Err(MetalError::CompileFailed(_))));
    }

    #[test]
    fn can_create_pipeline() {
        let d   = MetalDevice::new().unwrap();
        let lib = d.compile(MSL_IDENTITY).unwrap();
        let f   = lib.function("identity").unwrap();
        let pso = d.pipeline(&f);
        assert!(pso.is_ok(), "pipeline creation failed: {:?}", pso.err());
    }

    #[test]
    fn function_not_found_error() {
        let d   = MetalDevice::new().unwrap();
        let lib = d.compile(MSL_IDENTITY).unwrap();
        let res = lib.function("nonexistent_kernel");
        assert!(matches!(res, Err(MetalError::FunctionNotFound(_))));
    }

    #[test]
    fn alloc_and_read_buffer() {
        let d   = MetalDevice::new().unwrap();
        let src = vec![1u8, 2, 3, 4, 5];
        let mut buf = d.alloc_with_bytes(&src).unwrap();
        assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);

        // Write on CPU side.
        buf.as_slice_mut()[0] = 99;
        assert_eq!(buf.as_slice()[0], 99);
    }

    #[test]
    fn identity_kernel_round_trip() {
        let d      = MetalDevice::new().unwrap();
        let queue  = d.command_queue();
        let src    = (0u8..=255).collect::<Vec<u8>>();
        let n      = src.len() as u32;

        let src_buf = d.alloc_with_bytes(&src).unwrap();
        let dst_buf = d.alloc(src.len()).unwrap();

        let lib = d.compile(MSL_IDENTITY).unwrap();
        let f   = lib.function("identity").unwrap();
        let pso = d.pipeline(&f).unwrap();
        let tpg = pso.preferred_threads_1d();

        queue.dispatch(|enc| {
            enc.set_pipeline(&pso);
            enc.set_buffer(&src_buf, 0);
            enc.set_buffer(&dst_buf, 1);
            enc.set_bytes(n.to_le_bytes().as_ref(), 2);
            enc.dispatch_threads_1d(n, tpg);
        });

        assert_eq!(dst_buf.to_vec(), src);
    }

    #[test]
    fn invert_kernel() {
        let d      = MetalDevice::new().unwrap();
        let queue  = d.command_queue();
        let src    = vec![0u8, 128, 255, 64, 32];
        let n      = src.len() as u32;
        let expected: Vec<u8> = src.iter().map(|&b| 255 - b).collect();

        let src_buf = d.alloc_with_bytes(&src).unwrap();
        let dst_buf = d.alloc(src.len()).unwrap();

        let lib = d.compile(MSL_INVERT).unwrap();
        let f   = lib.function("invert").unwrap();
        let pso = d.pipeline(&f).unwrap();
        let tpg = pso.preferred_threads_1d();

        queue.dispatch(|enc| {
            enc.set_pipeline(&pso);
            enc.set_buffer(&src_buf, 0);
            enc.set_buffer(&dst_buf, 1);
            enc.set_bytes(n.to_le_bytes().as_ref(), 2);
            enc.dispatch_threads_1d(n, tpg);
        });

        assert_eq!(dst_buf.to_vec(), expected);
    }

    #[test]
    fn preferred_threads_1d_is_power_of_two() {
        let d   = MetalDevice::new().unwrap();
        let lib = d.compile(MSL_IDENTITY).unwrap();
        let f   = lib.function("identity").unwrap();
        let pso = d.pipeline(&f).unwrap();
        let t   = pso.preferred_threads_1d();
        assert!(t > 0);
        assert_eq!(t & (t - 1), 0, "{t} should be a power of two");
    }

    #[test]
    fn preferred_threads_2d_area_fits_max() {
        let d   = MetalDevice::new().unwrap();
        let lib = d.compile(MSL_IDENTITY).unwrap();
        let f   = lib.function("identity").unwrap();
        let pso = d.pipeline(&f).unwrap();
        let (w, h) = pso.preferred_threads_2d();
        assert!((w as u64) * (h as u64) <= pso.max_threads);
    }
}
