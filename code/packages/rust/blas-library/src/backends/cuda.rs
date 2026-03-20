//! CudaBlas -- NVIDIA CUDA BLAS backend.
//!
//! # How CudaBlas Works
//!
//! This backend wraps the `CudaRuntime` from Layer 4 (vendor-api-simulators).
//! For each BLAS operation, it follows the classic CUDA pattern:
//!
//! 1. `cuda.malloc()`         -- allocate device memory for inputs and output
//! 2. `cuda.memcpy(H2D)`     -- upload input data from host to device
//! 3. (compute)               -- perform the operation
//! 4. `cuda.memcpy(D2H)`     -- download results from device to host
//! 5. `cuda.free()`          -- release device memory
//!
//! Since our simulator's kernel execution is simplified, the actual arithmetic
//! is performed by the CPU reference (CpuBlas). The GPU memory pipeline is
//! fully exercised to demonstrate the CUDA programming pattern.

use vendor_api_simulators::cuda::CudaRuntime;

use super::gpu_base::GpuBlasBackend;

/// CUDA BLAS backend -- wraps CudaRuntime from Layer 4.
///
/// # CUDA BLAS -- NVIDIA GPU Acceleration
///
/// The most widely used GPU BLAS backend in ML. Real cuBLAS achieves
/// near-peak FLOPS on NVIDIA GPUs through:
/// - Tiled GEMM with shared memory
/// - Tensor Core acceleration (FP16/TF32)
/// - Warp-level matrix multiply (WMMA)
///
/// Our simulator demonstrates the memory management pattern:
/// cudaMalloc -> cudaMemcpy(H2D) -> compute -> cudaMemcpy(D2H) -> cudaFree
pub struct CudaBlas {
    cuda: CudaRuntime,
}

impl CudaBlas {
    /// Create a new CUDA BLAS backend.
    pub fn new() -> Result<Self, String> {
        let cuda = CudaRuntime::new()?;
        Ok(Self { cuda })
    }
}

impl GpuBlasBackend for CudaBlas {
    fn gpu_name(&self) -> &str {
        "cuda"
    }

    fn gpu_device_name(&self) -> String {
        self.cuda.get_device_properties().name
    }

    fn upload(&mut self, data: &[u8]) -> Result<usize, String> {
        let ptr = self.cuda.malloc(data.len())?;
        self.cuda.memcpy_host_to_device(ptr, data)?;
        Ok(ptr.buffer_id)
    }

    fn download(&mut self, handle: usize, size: usize) -> Result<Vec<u8>, String> {
        let mut host_buf = vec![0u8; size];
        let ptr = vendor_api_simulators::cuda::CudaDevicePtr {
            buffer_id: handle,
            device_address: 0,
            size,
        };
        self.cuda.memcpy_device_to_host(&mut host_buf, ptr)?;
        Ok(host_buf)
    }

    fn free(&mut self, handle: usize) -> Result<(), String> {
        let ptr = vendor_api_simulators::cuda::CudaDevicePtr {
            buffer_id: handle,
            device_address: 0,
            size: 0,
        };
        self.cuda.free(ptr)
    }
}
