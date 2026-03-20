// OpenGlBlas -- legacy OpenGL compute BLAS backend.
//
// # How OpenGlBlas Works
//
// This backend wraps GLContext from Layer 4. OpenGL uses a global state
// machine model -- you bind things to "current" state and then issue commands
// that operate on whatever is currently bound.
//
// For each BLAS operation:
//
//  1. gl.GenBuffers()       -- generate buffer IDs
//  2. gl.BindBuffer()       -- bind to a target
//  3. gl.BufferData()       -- allocate and upload data
//  4. (compute)             -- perform operation
//  5. gl.MapBufferRange()   -- map buffer for reading
//  6. gl.DeleteBuffers()    -- free buffers
//
// OpenGL compute shaders (4.3+) use Shader Storage Buffer Objects (SSBOs)
// for GPU-accessible storage.
package backends

import (
	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
	vas "github.com/adhithyan15/coding-adventures/code/packages/go/vendor-api-simulators"
)

// =========================================================================
// OpenGlBlas -- legacy state machine GPU acceleration
// =========================================================================

// OpenGlBlas wraps GLContext for BLAS operations. OpenGL is the oldest
// surviving GPU API (1992). Compute shaders were added in OpenGL 4.3 (2012),
// bolted onto the existing state machine model.
//
// The state machine means:
//   - glBindBuffer(target, id) sets "current buffer" globally
//   - glBufferData(target, ...) operates on WHATEVER is currently bound
//   - You must remember what is bound at all times
//
// Simple for small programs, error-prone for large ones.
//
// Usage:
//
//	gl, _ := NewOpenGlBlas()
//	result, _ := gl.Sgemm(NoTrans, NoTrans, 1.0, A, B, 0.0, C)
type OpenGlBlas struct {
	gpuBase
	gl *vas.GLContext
}

// NewOpenGlBlas creates a new OpenGL BLAS backend.
func NewOpenGlBlas() (*OpenGlBlas, error) {
	gl, err := vas.NewGLContext()
	if err != nil {
		return nil, err
	}
	ob := &OpenGlBlas{gl: gl}
	ob.gpuBase = newGpuBase(ob)
	return ob, nil
}

// Name returns the backend identifier.
func (o *OpenGlBlas) Name() string { return "opengl" }

// DeviceName returns a human-readable device name.
func (o *OpenGlBlas) DeviceName() string { return "OpenGL Device" }

// =========================================================================
// gpuMemory implementation -- OpenGL state machine memory operations
// =========================================================================

// upload creates an OpenGL SSBO and uploads data.
//
// OpenGL buffer creation follows the state machine pattern:
//  1. GenBuffers() -- get a buffer handle (integer)
//  2. BindBuffer(SHADER_STORAGE_BUFFER, handle) -- make it "current"
//  3. BufferData(SHADER_STORAGE_BUFFER, ...) -- allocate + upload data to current
func (o *OpenGlBlas) upload(data []byte) (interface{}, error) {
	bufIDs := o.gl.GenBuffers(1)
	bufID := bufIDs[0]
	if err := o.gl.BindBuffer(vas.GL_SHADER_STORAGE_BUFFER, bufID); err != nil {
		return nil, err
	}
	if err := o.gl.BufferData(vas.GL_SHADER_STORAGE_BUFFER, len(data), data, vas.GL_STATIC_DRAW); err != nil {
		return nil, err
	}
	return bufID, nil
}

// download maps the OpenGL buffer for reading and copies data out.
//
// OpenGL read-back also uses the state machine:
//  1. BindBuffer(SHADER_STORAGE_BUFFER, handle) -- make it current
//  2. MapBufferRange(SHADER_STORAGE_BUFFER, ..., MAP_READ_BIT) -- map for reading
//  3. Copy data out
//  4. UnmapBuffer() -- release the mapping
func (o *OpenGlBlas) download(handle interface{}, size int) ([]byte, error) {
	bufID := handle.(int)
	if err := o.gl.BindBuffer(vas.GL_SHADER_STORAGE_BUFFER, bufID); err != nil {
		return nil, err
	}
	mapped, err := o.gl.MapBufferRange(vas.GL_SHADER_STORAGE_BUFFER, 0, size, vas.GL_MAP_READ_BIT)
	if err != nil {
		return nil, err
	}
	result := make([]byte, size)
	copy(result, mapped[:size])
	o.gl.UnmapBuffer(vas.GL_SHADER_STORAGE_BUFFER)
	return result, nil
}

// free deletes the OpenGL buffer object.
func (o *OpenGlBlas) free(handle interface{}) error {
	bufID := handle.(int)
	o.gl.DeleteBuffers([]int{bufID})
	return nil
}

// Compile-time checks that OpenGlBlas implements both interfaces.
var _ blas.BlasBackend = (*OpenGlBlas)(nil)
var _ blas.MlBlasBackend = (*OpenGlBlas)(nil)
