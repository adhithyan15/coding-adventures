//! WebGpuBlas -- browser-friendly WebGPU BLAS backend.
//!
//! # How WebGpuBlas Works
//!
//! This backend wraps `GpuDevice` from Layer 4. WebGPU is designed for safe,
//! browser-based GPU compute with automatic synchronization.
//!
//! For each BLAS operation:
//! 1. `device.create_buffer(STORAGE | COPY_DST)` -- allocate with usage flags
//! 2. `device.queue_write_buffer()`               -- upload data
//! 3. (compute)                                    -- perform operation
//! 4. `device.read_buffer()`                       -- read results
//! 5. Buffer destroyed on drop
//!
//! WebGPU's key simplification: a single queue handles everything. No queue
//! families, no multiple queues.

use vendor_api_simulators::webgpu::{Gpu, GpuBufferDescriptor, GpuBufferUsage, GpuDevice};

use super::gpu_base::GpuBlasBackend;

/// WebGPU BLAS backend -- wraps GpuDevice from Layer 4.
///
/// # WebGPU BLAS -- Safe Browser-First GPU Acceleration
///
/// WebGPU provides a safe, validated GPU API designed for browsers:
/// - Single queue (device.queue)
/// - Automatic barriers (no manual synchronization)
/// - Usage-based buffer creation (STORAGE, COPY_SRC, COPY_DST, MAP_READ)
pub struct WebGpuBlas {
    device: GpuDevice,
    /// Track (buffer_id, size) for each uploaded buffer so we can
    /// reconstruct the proper buffer for download/free via `create_buffer`.
    buffer_registry: Vec<(usize, usize)>,
}

impl WebGpuBlas {
    /// Create a new WebGPU BLAS backend.
    pub fn new() -> Result<Self, String> {
        let gpu = Gpu::new()?;
        let adapter = gpu.request_adapter()?;
        let device = adapter.request_device()?;
        Ok(Self {
            device,
            buffer_registry: Vec::new(),
        })
    }
}

impl GpuBlasBackend for WebGpuBlas {
    fn gpu_name(&self) -> &str {
        "webgpu"
    }

    fn gpu_device_name(&self) -> String {
        "WebGPU Device".to_string()
    }

    fn upload(&mut self, data: &[u8]) -> Result<usize, String> {
        let desc = GpuBufferDescriptor {
            size: data.len(),
            usage: GpuBufferUsage::Storage,
            mapped_at_creation: false,
        };
        let buf = self.device.create_buffer(&desc)?;
        let buf_id = buf.buffer_id;
        self.device.queue_write_buffer(&buf, 0, data)?;
        // Track this buffer for later download/free operations.
        self.buffer_registry.push((buf_id, data.len()));
        // We intentionally forget the GpuBuffer struct -- the device memory
        // is still allocated and tracked by buf_id in our registry.
        std::mem::forget(buf);
        Ok(buf_id)
    }

    fn download(&mut self, handle: usize, size: usize) -> Result<Vec<u8>, String> {
        // Use the memory manager directly to read the buffer data, since
        // we cannot construct a GpuBuffer from outside the webgpu crate
        // (private fields).
        let mm = self.device.memory_manager_mut();
        mm.invalidate(handle, 0, 0)?;
        let data = {
            let mapped = mm.map(handle)?;
            mapped.read(0, size)?
        };
        mm.unmap(handle)?;
        Ok(data[..size].to_vec())
    }

    fn free(&mut self, handle: usize) -> Result<(), String> {
        // Free directly via the memory manager.
        let mm = self.device.memory_manager_mut();
        mm.free(handle)?;
        // Remove from our tracking registry.
        self.buffer_registry.retain(|(id, _)| *id != handle);
        Ok(())
    }
}
