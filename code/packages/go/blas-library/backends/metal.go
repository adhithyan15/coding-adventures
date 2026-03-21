// MetalBlas -- Apple Metal BLAS backend.
//
// # How MetalBlas Works
//
// This backend wraps MTLDevice from Layer 4. Metal's key advantage is
// unified memory -- on Apple Silicon, CPU and GPU share the same RAM.
// This means no host-to-device copies:
//
//	CUDA:   cudaMalloc -> cudaMemcpy(H2D) -> compute -> cudaMemcpy(D2H) -> cudaFree
//	Metal:  MakeBuffer -> WriteBytes       -> compute -> Contents()
//
// The buffer is always accessible from both CPU and GPU, so writes are
// immediate and reads require no copy.
//
// # Real Accelerate/MPS
//
// On real Apple hardware, Metal Performance Shaders (MPS) provides optimized
// BLAS operations that leverage the Apple GPU's unified memory architecture.
// PyTorch MPS backend uses this.
package backends

import (
	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
	vas "github.com/adhithyan15/coding-adventures/code/packages/go/vendor-api-simulators"
)

// =========================================================================
// MetalBlas -- Apple Silicon unified memory BLAS backend
// =========================================================================

// MetalBlas wraps MTLDevice for BLAS operations. Metal's unified memory
// model eliminates host-device copies:
//
//   - MakeBuffer() allocates memory visible to both CPU and GPU
//   - WriteBytes() writes directly (no staging buffer needed)
//   - Contents() reads directly (no download needed)
//
// This is the biggest ergonomic advantage of Apple Silicon for GPU computing.
//
// Usage:
//
//	metal, _ := NewMetalBlas()
//	result, _ := metal.Sgemm(NoTrans, NoTrans, 1.0, A, B, 0.0, C)
type MetalBlas struct {
	gpuBase
	device *vas.MTLDevice
}

// NewMetalBlas creates a new Metal BLAS backend by initializing the Metal device.
func NewMetalBlas() (*MetalBlas, error) {
	device, err := vas.NewMTLDevice()
	if err != nil {
		return nil, err
	}
	mb := &MetalBlas{device: device}
	mb.gpuBase = newGpuBase(mb)
	return mb, nil
}

// Name returns the backend identifier.
func (m *MetalBlas) Name() string { return "metal" }

// DeviceName returns a human-readable device name.
func (m *MetalBlas) DeviceName() string { return m.device.Name() }

// =========================================================================
// gpuMemory implementation -- Metal unified memory operations
// =========================================================================

// upload creates a Metal buffer with unified memory and writes data.
//
// On Apple Silicon, this buffer lives in unified RAM accessible from both
// CPU and GPU. No explicit host-to-device copy is needed.
func (m *MetalBlas) upload(data []byte) (interface{}, error) {
	buf, err := m.device.MakeBuffer(len(data), vas.MTLResourceStorageModeShared)
	if err != nil {
		return nil, err
	}
	if err := buf.WriteBytes(data, 0); err != nil {
		return nil, err
	}
	return buf, nil
}

// download reads directly from the Metal buffer (unified memory).
//
// Because Metal uses unified memory, Contents() provides direct CPU access
// to the buffer data without any GPU-to-CPU copy.
func (m *MetalBlas) download(handle interface{}, size int) ([]byte, error) {
	buf := handle.(*vas.MTLBuffer)
	contents, err := buf.Contents()
	if err != nil {
		return nil, err
	}
	result := make([]byte, size)
	copy(result, contents[:size])
	return result, nil
}

// free is a no-op for Metal. Metal uses automatic reference counting (ARC),
// so buffers are deallocated when no references remain.
func (m *MetalBlas) free(handle interface{}) error {
	return nil
}

// Compile-time checks that MetalBlas implements both interfaces.
var _ blas.BlasBackend = (*MetalBlas)(nil)
var _ blas.MlBlasBackend = (*MetalBlas)(nil)
