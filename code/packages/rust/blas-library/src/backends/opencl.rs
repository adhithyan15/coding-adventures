//! OpenClBlas -- portable OpenCL BLAS backend.
//!
//! # How OpenClBlas Works
//!
//! This backend wraps `ClContext` and `ClCommandQueue` from Layer 4.
//! OpenCL's distinctive feature is event-based dependencies -- every enqueue
//! operation returns a CLEvent that subsequent operations can wait on.
//!
//! For each BLAS operation:
//! 1. `ctx.create_buffer()`            -- allocate device memory
//! 2. `queue.enqueue_write_buffer()`   -- upload data (returns event)
//! 3. (compute)                         -- perform the operation
//! 4. `queue.enqueue_read_buffer()`    -- download results
//! 5. `queue.finish()`                 -- wait for all operations
//!
//! OpenCL is the most portable GPU API -- it runs on NVIDIA, AMD, Intel
//! GPUs, and even CPUs and FPGAs.

use vendor_api_simulators::opencl::{ClContext, ClMemFlags};

use super::gpu_base::GpuBlasBackend;

/// OpenCL BLAS backend -- wraps ClContext from Layer 4.
///
/// # OpenCL BLAS -- Portable GPU Acceleration
///
/// OpenCL (Open Computing Language) is the Khronos Group's cross-platform
/// compute API. Unlike CUDA (NVIDIA only), OpenCL runs on any vendor's
/// GPU and even on CPUs.
pub struct OpenClBlas {
    ctx: ClContext,
}

impl OpenClBlas {
    /// Create a new OpenCL BLAS backend.
    pub fn new() -> Result<Self, String> {
        let ctx = ClContext::new()?;
        Ok(Self { ctx })
    }
}

impl GpuBlasBackend for OpenClBlas {
    fn gpu_name(&self) -> &str {
        "opencl"
    }

    fn gpu_device_name(&self) -> String {
        self.ctx.devices()[0].name.clone()
    }

    fn upload(&mut self, data: &[u8]) -> Result<usize, String> {
        let buf = self.ctx.create_buffer(ClMemFlags::ReadWrite, data.len(), Some(data))?;
        Ok(buf.buffer_id)
    }

    fn download(&mut self, handle: usize, size: usize) -> Result<Vec<u8>, String> {
        let mut host_buf = vec![0u8; size];
        let buf = vendor_api_simulators::opencl::ClBuffer {
            buffer_id: handle,
            size,
            flags: ClMemFlags::ReadWrite,
        };
        let mut queue = self.ctx.create_command_queue();
        queue.enqueue_read_buffer(&buf, 0, size, &mut host_buf, &[])?;
        queue.finish();
        Ok(host_buf)
    }

    fn free(&mut self, _handle: usize) -> Result<(), String> {
        // OpenCL buffers are freed when the context is destroyed.
        Ok(())
    }
}
