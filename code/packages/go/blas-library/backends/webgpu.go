// WebGpuBlas -- browser-friendly WebGPU BLAS backend.
//
// # How WebGpuBlas Works
//
// This backend wraps GPUDevice from Layer 4. WebGPU is designed for safe,
// browser-based GPU compute with automatic synchronization.
//
// For each BLAS operation:
//
//  1. device.CreateBuffer(STORAGE | COPY_DST)  -- allocate with usage flags
//  2. device.Queue.WriteBuffer()               -- upload data
//  3. (compute)                                 -- perform operation
//  4. Create a MAP_READ staging buffer, copy, map, read
//  5. buffer.Destroy() (auto-freed)
//
// WebGPU's key simplification: a single queue (device.Queue) handles
// everything. No queue families, no multiple queues.
package backends

import (
	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
	vas "github.com/adhithyan15/coding-adventures/code/packages/go/vendor-api-simulators"
)

// =========================================================================
// WebGpuBlas -- safe browser-first GPU acceleration
// =========================================================================

// WebGpuBlas wraps GPUDevice for BLAS operations. WebGPU provides a safe,
// validated GPU API designed for browsers:
//
//   - Single queue (device.Queue)
//   - Automatic barriers (no manual synchronization)
//   - Usage-based buffer creation (STORAGE, COPY_SRC, COPY_DST, MAP_READ)
//
// Usage:
//
//	wg, _ := NewWebGpuBlas()
//	result, _ := wg.Sgemm(NoTrans, NoTrans, 1.0, A, B, 0.0, C)
type WebGpuBlas struct {
	gpuBase
	device *vas.GPUDevice
}

// NewWebGpuBlas creates a new WebGPU BLAS backend.
//
// The WebGPU initialization sequence mirrors the browser API:
//
//  1. Create a GPU instance (navigator.gpu)
//  2. Request an adapter (physical device selection)
//  3. Request a device (the usable handle)
func NewWebGpuBlas() (*WebGpuBlas, error) {
	gpu, err := vas.NewGPU()
	if err != nil {
		return nil, err
	}
	adapter, err := gpu.RequestAdapter(nil)
	if err != nil {
		return nil, err
	}
	device, err := adapter.RequestDevice()
	if err != nil {
		return nil, err
	}
	wb := &WebGpuBlas{device: device}
	wb.gpuBase = newGpuBase(wb)
	return wb, nil
}

// Name returns the backend identifier.
func (w *WebGpuBlas) Name() string { return "webgpu" }

// DeviceName returns a human-readable device name.
func (w *WebGpuBlas) DeviceName() string { return "WebGPU Device" }

// =========================================================================
// gpuMemory implementation -- WebGPU usage-based memory operations
// =========================================================================

// upload creates a WebGPU buffer with STORAGE usage and writes data.
//
// WebGPU buffers require explicit usage flags at creation time. We request
// STORAGE (for compute shaders), COPY_DST (for queue.WriteBuffer), and
// COPY_SRC (for readback copying).
func (w *WebGpuBlas) upload(data []byte) (interface{}, error) {
	desc := vas.GPUBufferDescriptor{
		Size:  len(data),
		Usage: vas.GPUBufferUsageStorage | vas.GPUBufferUsageCopyDst | vas.GPUBufferUsageCopySrc,
	}
	buf, err := w.device.CreateBuffer(desc)
	if err != nil {
		return nil, err
	}
	if err := w.device.Queue.WriteBuffer(buf, 0, data); err != nil {
		return nil, err
	}
	return buf, nil
}

// download creates a MAP_READ staging buffer, copies data, maps, and reads.
//
// WebGPU does not allow direct CPU reading from STORAGE buffers. Instead,
// you must:
//  1. Create a staging buffer with MAP_READ | COPY_DST
//  2. Copy from the source buffer to the staging buffer
//  3. Map the staging buffer for reading
//  4. Read the mapped range
//  5. Unmap the staging buffer
func (w *WebGpuBlas) download(handle interface{}, size int) ([]byte, error) {
	source := handle.(*vas.GPUBuffer)

	// Create staging buffer for readback
	stagingDesc := vas.GPUBufferDescriptor{
		Size:  size,
		Usage: vas.GPUBufferUsageMapRead | vas.GPUBufferUsageCopyDst,
	}
	staging, err := w.device.CreateBuffer(stagingDesc)
	if err != nil {
		return nil, err
	}

	// Copy from source to staging via a command encoder
	encoder, err := w.device.CreateCommandEncoder()
	if err != nil {
		return nil, err
	}
	if err := encoder.CopyBufferToBuffer(source, 0, staging, 0, size); err != nil {
		return nil, err
	}
	cmdBuf, err := encoder.Finish()
	if err != nil {
		return nil, err
	}
	if err := w.device.Queue.Submit([]*vas.GPUCommandBuffer{cmdBuf}); err != nil {
		return nil, err
	}

	// Map and read the staging buffer
	if err := staging.MapAsync(vas.GPUMapModeRead, 0, 0); err != nil {
		return nil, err
	}
	data, err := staging.GetMappedRange(0, size)
	if err != nil {
		return nil, err
	}
	result := make([]byte, size)
	copy(result, data)
	_ = staging.Unmap()
	return result, nil
}

// free destroys the WebGPU buffer, releasing GPU memory.
func (w *WebGpuBlas) free(handle interface{}) error {
	buf := handle.(*vas.GPUBuffer)
	return buf.Destroy()
}

// Compile-time checks that WebGpuBlas implements both interfaces.
var _ blas.BlasBackend = (*WebGpuBlas)(nil)
var _ blas.MlBlasBackend = (*WebGpuBlas)(nil)
