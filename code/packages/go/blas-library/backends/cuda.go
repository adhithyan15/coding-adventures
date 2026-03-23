// CudaBlas -- NVIDIA CUDA BLAS backend.
//
// # How CudaBlas Works
//
// This backend wraps the CUDARuntime from Layer 4 (vendor-api-simulators).
// For each BLAS operation, it follows the classic CUDA pattern:
//
//  1. cudaMalloc()       -- allocate device memory for inputs and output
//  2. cudaMemcpy(H2D)   -- upload input data from host to device
//  3. (compute)          -- perform the operation (via CPU reference)
//  4. cudaMemcpy(D2H)   -- download results from device to host
//  5. cudaFree()         -- release device memory
//
// Since our simulator's kernel execution is simplified, the actual arithmetic
// is performed by the CPU reference (CpuBlas). The GPU memory pipeline is
// fully exercised to demonstrate the CUDA programming pattern.
//
// # Real cuBLAS
//
// In the real world, cublasSgemm() launches highly optimized CUDA kernels
// that tile the computation across thousands of GPU threads, using shared
// memory, warp-level primitives, and tensor cores. Our simulator demonstrates
// the memory management pattern without that complexity.
package backends

import (
	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
	vas "github.com/adhithyan15/coding-adventures/code/packages/go/vendor-api-simulators"
)

// =========================================================================
// CudaBlas -- NVIDIA GPU BLAS backend
// =========================================================================

// CudaBlas wraps CUDARuntime for BLAS operations. It embeds gpuBase which
// provides all BLAS operations via the template method pattern.
//
// The memory flow for each operation:
//
//	cudaMalloc -> cudaMemcpy(H2D) -> compute -> cudaMemcpy(D2H) -> cudaFree
//
// Usage:
//
//	cuda, _ := NewCudaBlas()
//	result, _ := cuda.Sgemm(NoTrans, NoTrans, 1.0, A, B, 0.0, C)
type CudaBlas struct {
	gpuBase
	cuda *vas.CUDARuntime
}

// NewCudaBlas creates a new CUDA BLAS backend by initializing the CUDA runtime.
func NewCudaBlas() (*CudaBlas, error) {
	cuda, err := vas.NewCUDARuntime()
	if err != nil {
		return nil, err
	}
	cb := &CudaBlas{cuda: cuda}
	cb.gpuBase = newGpuBase(cb)
	return cb, nil
}

// Name returns the backend identifier.
func (c *CudaBlas) Name() string { return "cuda" }

// DeviceName returns a human-readable device name from CUDA properties.
func (c *CudaBlas) DeviceName() string {
	props := c.cuda.GetDeviceProperties()
	return props.Name
}

// =========================================================================
// gpuMemory implementation -- CUDA-specific memory operations
// =========================================================================

// upload allocates GPU memory via cudaMalloc and copies data via cudaMemcpy(H2D).
func (c *CudaBlas) upload(data []byte) (interface{}, error) {
	ptr, err := c.cuda.Malloc(len(data))
	if err != nil {
		return nil, err
	}
	err = c.cuda.Memcpy(ptr, nil, data, nil, len(data), vas.CUDAMemcpyHostToDevice)
	if err != nil {
		return nil, err
	}
	return ptr, nil
}

// download reads data from GPU via cudaMemcpy(D2H).
func (c *CudaBlas) download(handle interface{}, size int) ([]byte, error) {
	ptr := handle.(*vas.CUDADevicePtr)
	hostBuf := make([]byte, size)
	err := c.cuda.Memcpy(nil, hostBuf, nil, ptr, size, vas.CUDAMemcpyDeviceToHost)
	if err != nil {
		return nil, err
	}
	return hostBuf, nil
}

// free releases GPU memory via cudaFree.
func (c *CudaBlas) free(handle interface{}) error {
	ptr := handle.(*vas.CUDADevicePtr)
	return c.cuda.Free(ptr)
}

// Compile-time checks that CudaBlas implements both interfaces.
var _ blas.BlasBackend = (*CudaBlas)(nil)
var _ blas.MlBlasBackend = (*CudaBlas)(nil)
