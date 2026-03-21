// OpenClBlas -- portable OpenCL BLAS backend.
//
// # How OpenClBlas Works
//
// This backend wraps CLContext and CLCommandQueue from Layer 4. OpenCL's
// distinctive feature is event-based dependencies -- every enqueue operation
// returns a CLEvent that subsequent operations can wait on.
//
// For each BLAS operation:
//
//  1. ctx.CreateBuffer()            -- allocate device memory
//  2. queue.EnqueueWriteBuffer()    -- upload data (returns event)
//  3. (compute)                     -- perform the operation
//  4. queue.EnqueueReadBuffer()     -- download results
//  5. queue.Finish()                -- wait for all operations
//
// OpenCL is the most portable GPU API -- it runs on NVIDIA, AMD, Intel GPUs,
// and even CPUs and FPGAs.
package backends

import (
	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
	vas "github.com/adhithyan15/coding-adventures/code/packages/go/vendor-api-simulators"
)

// =========================================================================
// OpenClBlas -- portable GPU acceleration
// =========================================================================

// OpenClBlas wraps CLContext for BLAS operations. OpenCL (Open Computing
// Language) is the Khronos Group's cross-platform compute API. Unlike CUDA
// (NVIDIA only), OpenCL runs on any vendor's GPU and even on CPUs.
//
// Our simulator exercises the OpenCL memory pipeline:
//
//	create_buffer -> enqueue_write -> compute -> enqueue_read -> finish
//
// Usage:
//
//	cl, _ := NewOpenClBlas()
//	result, _ := cl.Sgemm(NoTrans, NoTrans, 1.0, A, B, 0.0, C)
type OpenClBlas struct {
	gpuBase
	ctx   *vas.CLContext
	queue *vas.CLCommandQueue
}

// NewOpenClBlas creates a new OpenCL BLAS backend.
func NewOpenClBlas() (*OpenClBlas, error) {
	ctx, err := vas.NewCLContext(nil)
	if err != nil {
		return nil, err
	}
	queue := ctx.CreateCommandQueue(nil)
	cl := &OpenClBlas{ctx: ctx, queue: queue}
	cl.gpuBase = newGpuBase(cl)
	return cl, nil
}

// Name returns the backend identifier.
func (o *OpenClBlas) Name() string { return "opencl" }

// DeviceName returns a human-readable device name.
func (o *OpenClBlas) DeviceName() string {
	devices := o.ctx.Devices()
	if len(devices) > 0 {
		return devices[0].Name()
	}
	return "OpenCL Device"
}

// =========================================================================
// gpuMemory implementation -- OpenCL event-based memory operations
// =========================================================================

// upload creates a CLBuffer and uploads data via enqueue_write_buffer.
func (o *OpenClBlas) upload(data []byte) (interface{}, error) {
	buf, err := o.ctx.CreateBuffer(vas.CLMemReadWrite, len(data), nil)
	if err != nil {
		return nil, err
	}
	_, err = o.queue.EnqueueWriteBuffer(buf, 0, len(data), data, nil)
	if err != nil {
		return nil, err
	}
	return buf, nil
}

// download reads data via enqueue_read_buffer and finish.
func (o *OpenClBlas) download(handle interface{}, size int) ([]byte, error) {
	buf := handle.(*vas.CLBuffer)
	hostBuf := make([]byte, size)
	_, err := o.queue.EnqueueReadBuffer(buf, 0, size, hostBuf, nil)
	if err != nil {
		return nil, err
	}
	o.queue.Finish()
	return hostBuf, nil
}

// free is a no-op for OpenCL. CLBuffer doesn't have an explicit free --
// buffers are garbage collected with the context.
func (o *OpenClBlas) free(handle interface{}) error {
	return nil
}

// Compile-time checks that OpenClBlas implements both interfaces.
var _ blas.BlasBackend = (*OpenClBlas)(nil)
var _ blas.MlBlasBackend = (*OpenClBlas)(nil)
