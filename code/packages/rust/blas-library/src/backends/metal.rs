//! MetalBlas -- Apple Metal BLAS backend.
//!
//! # How MetalBlas Works
//!
//! This backend wraps `MtlDevice` from Layer 4. Metal's key advantage is
//! **unified memory** -- on Apple Silicon, CPU and GPU share the same RAM.
//! This means no host-to-device copies:
//!
//! ```text
//! CUDA:   cudaMalloc -> cudaMemcpy(H2D) -> compute -> cudaMemcpy(D2H) -> cudaFree
//! Metal:  make_buffer -> write_bytes     -> compute -> read_buffer()
//! ```
//!
//! The buffer is always accessible from both CPU and GPU, so writes are
//! immediate and reads require no copy.

use vendor_api_simulators::metal::MtlDevice;

use super::gpu_base::GpuBlasBackend;

/// Metal BLAS backend -- wraps MtlDevice from Layer 4.
///
/// # Metal BLAS -- Apple Silicon Unified Memory
///
/// Metal's unified memory model eliminates host-device copies:
/// - `make_buffer()` allocates memory visible to both CPU and GPU
/// - `write_buffer()` writes directly (no staging buffer needed)
/// - `read_buffer()` reads directly (no download needed)
pub struct MetalBlas {
    device: MtlDevice,
}

impl MetalBlas {
    /// Create a new Metal BLAS backend.
    pub fn new() -> Result<Self, String> {
        let device = MtlDevice::new()?;
        Ok(Self { device })
    }
}

impl GpuBlasBackend for MetalBlas {
    fn gpu_name(&self) -> &str {
        "metal"
    }

    fn gpu_device_name(&self) -> String {
        self.device.name()
    }

    fn upload(&mut self, data: &[u8]) -> Result<usize, String> {
        let buf = self.device.make_buffer(data.len())?;
        let buf_id = buf.buffer_id;
        self.device.write_buffer(&buf, data)?;
        Ok(buf_id)
    }

    fn download(&mut self, handle: usize, size: usize) -> Result<Vec<u8>, String> {
        let buf = vendor_api_simulators::metal::MtlBuffer {
            buffer_id: handle,
            length: size,
        };
        let data = self.device.read_buffer(&buf)?;
        Ok(data[..size].to_vec())
    }

    fn free(&mut self, handle: usize) -> Result<(), String> {
        let buf = vendor_api_simulators::metal::MtlBuffer {
            buffer_id: handle,
            length: 0,
        };
        self.device.release_buffer(buf)
    }
}
