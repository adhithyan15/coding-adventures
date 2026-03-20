package vendorapisimulators

import (
	"testing"
)

// =========================================================================
// OpenGL tests
// =========================================================================

func newGL(t *testing.T) *GLContext {
	t.Helper()
	gl, err := NewGLContext()
	if err != nil {
		t.Fatalf("NewGLContext failed: %v", err)
	}
	return gl
}

func TestGLContextCreation(t *testing.T) {
	gl := newGL(t)
	if gl.LogicalDevice == nil {
		t.Fatal("expected non-nil LogicalDevice")
	}
	if gl.currentProgram != 0 {
		t.Error("expected no current program initially")
	}
}

func TestGLCreateShader(t *testing.T) {
	gl := newGL(t)
	handle, err := gl.CreateShader(GL_COMPUTE_SHADER)
	if err != nil {
		t.Fatalf("CreateShader failed: %v", err)
	}
	if handle <= 0 {
		t.Errorf("expected positive handle, got %d", handle)
	}
}

func TestGLCreateShaderInvalidType(t *testing.T) {
	gl := newGL(t)
	_, err := gl.CreateShader(0x1234)
	if err == nil {
		t.Error("expected error for invalid shader type")
	}
}

func TestGLShaderSource(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	err := gl.ShaderSource(shader, "void main() {}")
	if err != nil {
		t.Fatalf("ShaderSource failed: %v", err)
	}
}

func TestGLShaderSourceInvalidHandle(t *testing.T) {
	gl := newGL(t)
	err := gl.ShaderSource(9999, "src")
	if err == nil {
		t.Error("expected error for invalid shader handle")
	}
}

func TestGLCompileShader(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.ShaderSource(shader, "void main() {}")
	err := gl.CompileShader(shader)
	if err != nil {
		t.Fatalf("CompileShader failed: %v", err)
	}
}

func TestGLCompileShaderInvalidHandle(t *testing.T) {
	gl := newGL(t)
	err := gl.CompileShader(9999)
	if err == nil {
		t.Error("expected error for invalid shader handle")
	}
}

func TestGLDeleteShader(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.DeleteShader(shader)
	// Verify deleted
	err := gl.ShaderSource(shader, "src")
	if err == nil {
		t.Error("expected error after shader deletion")
	}
}

func TestGLCreateProgram(t *testing.T) {
	gl := newGL(t)
	prog := gl.CreateProgram()
	if prog <= 0 {
		t.Errorf("expected positive handle, got %d", prog)
	}
}

func TestGLAttachShader(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	prog := gl.CreateProgram()
	err := gl.AttachShader(prog, shader)
	if err != nil {
		t.Fatalf("AttachShader failed: %v", err)
	}
}

func TestGLAttachShaderInvalidProgram(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	err := gl.AttachShader(9999, shader)
	if err == nil {
		t.Error("expected error for invalid program handle")
	}
}

func TestGLAttachShaderInvalidShader(t *testing.T) {
	gl := newGL(t)
	prog := gl.CreateProgram()
	err := gl.AttachShader(prog, 9999)
	if err == nil {
		t.Error("expected error for invalid shader handle")
	}
}

func TestGLLinkProgram(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.CompileShader(shader)
	prog := gl.CreateProgram()
	gl.AttachShader(prog, shader)
	err := gl.LinkProgram(prog)
	if err != nil {
		t.Fatalf("LinkProgram failed: %v", err)
	}
}

func TestGLLinkProgramInvalid(t *testing.T) {
	gl := newGL(t)
	err := gl.LinkProgram(9999)
	if err == nil {
		t.Error("expected error for invalid program handle")
	}
}

func TestGLLinkProgramNoShaders(t *testing.T) {
	gl := newGL(t)
	prog := gl.CreateProgram()
	err := gl.LinkProgram(prog)
	if err == nil {
		t.Error("expected error for program with no shaders")
	}
}

func TestGLUseProgram(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.CompileShader(shader)
	prog := gl.CreateProgram()
	gl.AttachShader(prog, shader)
	gl.LinkProgram(prog)

	err := gl.UseProgram(prog)
	if err != nil {
		t.Fatalf("UseProgram failed: %v", err)
	}
	if gl.currentProgram != prog {
		t.Errorf("expected current program %d, got %d", prog, gl.currentProgram)
	}
}

func TestGLUseProgramZero(t *testing.T) {
	gl := newGL(t)
	err := gl.UseProgram(0)
	if err != nil {
		t.Fatalf("UseProgram(0) failed: %v", err)
	}
	if gl.currentProgram != 0 {
		t.Error("expected no current program")
	}
}

func TestGLUseProgramInvalid(t *testing.T) {
	gl := newGL(t)
	err := gl.UseProgram(9999)
	if err == nil {
		t.Error("expected error for invalid program handle")
	}
}

func TestGLUseProgramNotLinked(t *testing.T) {
	gl := newGL(t)
	prog := gl.CreateProgram()
	err := gl.UseProgram(prog)
	if err == nil {
		t.Error("expected error for unlinked program")
	}
}

func TestGLDeleteProgram(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.CompileShader(shader)
	prog := gl.CreateProgram()
	gl.AttachShader(prog, shader)
	gl.LinkProgram(prog)
	gl.UseProgram(prog)

	gl.DeleteProgram(prog)
	if gl.currentProgram != 0 {
		t.Error("expected current program to be cleared after delete")
	}
}

func TestGLGenBuffers(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(3)
	if len(bufs) != 3 {
		t.Errorf("expected 3 buffers, got %d", len(bufs))
	}
	for _, b := range bufs {
		if b <= 0 {
			t.Errorf("expected positive handle, got %d", b)
		}
	}
}

func TestGLBindBuffer(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	err := gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	if err != nil {
		t.Fatalf("BindBuffer failed: %v", err)
	}
}

func TestGLBindBufferZero(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	err := gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, 0) // unbind
	if err != nil {
		t.Fatalf("BindBuffer(0) failed: %v", err)
	}
}

func TestGLBindBufferInvalid(t *testing.T) {
	gl := newGL(t)
	err := gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, 9999)
	if err == nil {
		t.Error("expected error for invalid buffer handle")
	}
}

func TestGLBufferData(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	err := gl.BufferData(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_DYNAMIC_DRAW)
	if err != nil {
		t.Fatalf("BufferData failed: %v", err)
	}
}

func TestGLBufferDataWithInitialData(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	data := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	err := gl.BufferData(GL_SHADER_STORAGE_BUFFER, 8, data, GL_STATIC_DRAW)
	if err != nil {
		t.Fatalf("BufferData with data failed: %v", err)
	}
}

func TestGLBufferDataNoBufferBound(t *testing.T) {
	gl := newGL(t)
	err := gl.BufferData(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_STATIC_DRAW)
	if err == nil {
		t.Error("expected error with no buffer bound to target")
	}
}

func TestGLBufferDataReallocation(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	gl.BufferData(GL_SHADER_STORAGE_BUFFER, 32, nil, GL_DYNAMIC_DRAW)
	// Reallocate with new size
	err := gl.BufferData(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_DYNAMIC_DRAW)
	if err != nil {
		t.Fatalf("BufferData reallocation failed: %v", err)
	}
}

func TestGLBufferSubData(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	gl.BufferData(GL_SHADER_STORAGE_BUFFER, 16, nil, GL_DYNAMIC_DRAW)
	err := gl.BufferSubData(GL_SHADER_STORAGE_BUFFER, 4, []byte{0xAA, 0xBB})
	if err != nil {
		t.Fatalf("BufferSubData failed: %v", err)
	}
}

func TestGLBufferSubDataNoBufferBound(t *testing.T) {
	gl := newGL(t)
	err := gl.BufferSubData(GL_SHADER_STORAGE_BUFFER, 0, []byte{1})
	if err == nil {
		t.Error("expected error with no buffer bound")
	}
}

func TestGLBufferSubDataNoDataStore(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	// No BufferData call yet
	err := gl.BufferSubData(GL_SHADER_STORAGE_BUFFER, 0, []byte{1})
	if err == nil {
		t.Error("expected error for buffer with no data store")
	}
}

func TestGLBindBufferBase(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	gl.BufferData(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_DYNAMIC_DRAW)
	err := gl.BindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])
	if err != nil {
		t.Fatalf("BindBufferBase failed: %v", err)
	}
}

func TestGLBindBufferBaseInvalid(t *testing.T) {
	gl := newGL(t)
	err := gl.BindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, 9999)
	if err == nil {
		t.Error("expected error for invalid buffer handle")
	}
}

func TestGLMapBufferRange(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	gl.BufferData(GL_SHADER_STORAGE_BUFFER, 32, []byte{10, 20, 30, 40}, GL_DYNAMIC_DRAW)

	data, err := gl.MapBufferRange(GL_SHADER_STORAGE_BUFFER, 0, 4, GL_MAP_READ_BIT)
	if err != nil {
		t.Fatalf("MapBufferRange failed: %v", err)
	}
	if len(data) < 4 {
		t.Fatal("expected at least 4 bytes")
	}
}

func TestGLMapBufferRangeNoBound(t *testing.T) {
	gl := newGL(t)
	_, err := gl.MapBufferRange(GL_SHADER_STORAGE_BUFFER, 0, 4, GL_MAP_READ_BIT)
	if err == nil {
		t.Error("expected error with no buffer bound")
	}
}

func TestGLMapBufferRangeNoDataStore(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	_, err := gl.MapBufferRange(GL_SHADER_STORAGE_BUFFER, 0, 4, GL_MAP_READ_BIT)
	if err == nil {
		t.Error("expected error for buffer with no data store")
	}
}

func TestGLUnmapBuffer(t *testing.T) {
	gl := newGL(t)
	result := gl.UnmapBuffer(GL_SHADER_STORAGE_BUFFER)
	if !result {
		t.Error("expected true from UnmapBuffer")
	}
}

func TestGLDeleteBuffers(t *testing.T) {
	gl := newGL(t)
	bufs := gl.GenBuffers(2)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	gl.BufferData(GL_SHADER_STORAGE_BUFFER, 32, nil, GL_DYNAMIC_DRAW)
	gl.BindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])

	gl.DeleteBuffers(bufs)
	// Should be able to verify bindings were cleaned up
	err := gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	if err == nil {
		t.Error("expected error for deleted buffer handle")
	}
}

func TestGLDispatchCompute(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.CompileShader(shader)
	prog := gl.CreateProgram()
	gl.AttachShader(prog, shader)
	gl.LinkProgram(prog)
	gl.UseProgram(prog)

	bufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
	gl.BufferData(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_DYNAMIC_DRAW)
	gl.BindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])

	err := gl.DispatchCompute(4, 1, 1)
	if err != nil {
		t.Fatalf("DispatchCompute failed: %v", err)
	}
}

func TestGLDispatchComputeNoProgram(t *testing.T) {
	gl := newGL(t)
	err := gl.DispatchCompute(1, 1, 1)
	if err == nil {
		t.Error("expected error for dispatch without active program")
	}
}

func TestGLDispatchComputeMultipleSSBOs(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.CompileShader(shader)
	prog := gl.CreateProgram()
	gl.AttachShader(prog, shader)
	gl.LinkProgram(prog)
	gl.UseProgram(prog)

	bufs := gl.GenBuffers(3)
	for i, b := range bufs {
		gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, b)
		gl.BufferData(GL_SHADER_STORAGE_BUFFER, 32, nil, GL_DYNAMIC_DRAW)
		gl.BindBufferBase(GL_SHADER_STORAGE_BUFFER, i, b)
	}

	err := gl.DispatchCompute(2, 2, 1)
	if err != nil {
		t.Fatalf("DispatchCompute with multiple SSBOs failed: %v", err)
	}
}

func TestGLMemoryBarrier(t *testing.T) {
	gl := newGL(t)
	gl.MemoryBarrier(GL_SHADER_STORAGE_BARRIER_BIT) // Should not panic
	gl.MemoryBarrier(GL_ALL_BARRIER_BITS)            // Should not panic
}

func TestGLFenceSync(t *testing.T) {
	gl := newGL(t)
	sync := gl.FenceSync()
	if sync <= 0 {
		t.Errorf("expected positive sync handle, got %d", sync)
	}
}

func TestGLClientWaitSync(t *testing.T) {
	gl := newGL(t)
	sync := gl.FenceSync()
	result := gl.ClientWaitSync(sync, GL_SYNC_FLUSH_COMMANDS_BIT, 1000)
	if result != GL_ALREADY_SIGNALED {
		t.Errorf("expected GL_ALREADY_SIGNALED, got 0x%04X", result)
	}
}

func TestGLClientWaitSyncInvalid(t *testing.T) {
	gl := newGL(t)
	result := gl.ClientWaitSync(9999, 0, 0)
	if result != GL_WAIT_FAILED {
		t.Errorf("expected GL_WAIT_FAILED, got 0x%04X", result)
	}
}

func TestGLDeleteSync(t *testing.T) {
	gl := newGL(t)
	sync := gl.FenceSync()
	gl.DeleteSync(sync)
	// After delete, should fail
	result := gl.ClientWaitSync(sync, 0, 0)
	if result != GL_WAIT_FAILED {
		t.Errorf("expected GL_WAIT_FAILED after delete, got 0x%04X", result)
	}
}

func TestGLFinish(t *testing.T) {
	gl := newGL(t)
	gl.Finish() // Should not panic
}

func TestGLGetUniformLocation(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.CompileShader(shader)
	prog := gl.CreateProgram()
	gl.AttachShader(prog, shader)
	gl.LinkProgram(prog)

	loc, err := gl.GetUniformLocation(prog, "scale")
	if err != nil {
		t.Fatalf("GetUniformLocation failed: %v", err)
	}
	if loc < 0 {
		t.Errorf("expected non-negative location, got %d", loc)
	}
}

func TestGLGetUniformLocationInvalid(t *testing.T) {
	gl := newGL(t)
	_, err := gl.GetUniformLocation(9999, "x")
	if err == nil {
		t.Error("expected error for invalid program handle")
	}
}

func TestGLUniform1f(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.CompileShader(shader)
	prog := gl.CreateProgram()
	gl.AttachShader(prog, shader)
	gl.LinkProgram(prog)
	gl.UseProgram(prog)

	loc, _ := gl.GetUniformLocation(prog, "scale")
	gl.Uniform1f(loc, 3.14)
	// Verify it was stored
	val, ok := gl.uniforms[[2]interface{}{prog, loc}]
	if !ok {
		t.Error("expected uniform to be stored")
	}
	if val != 3.14 {
		t.Errorf("expected 3.14, got %v", val)
	}
}

func TestGLUniform1fNoProgram(t *testing.T) {
	gl := newGL(t)
	// With no program active, should silently do nothing
	gl.Uniform1f(0, 1.0)
}

func TestGLUniform1i(t *testing.T) {
	gl := newGL(t)
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.CompileShader(shader)
	prog := gl.CreateProgram()
	gl.AttachShader(prog, shader)
	gl.LinkProgram(prog)
	gl.UseProgram(prog)

	loc, _ := gl.GetUniformLocation(prog, "count")
	gl.Uniform1i(loc, 42)
	val, ok := gl.uniforms[[2]interface{}{prog, loc}]
	if !ok {
		t.Error("expected uniform to be stored")
	}
	if val != 42 {
		t.Errorf("expected 42, got %v", val)
	}
}

func TestGLConstants(t *testing.T) {
	if GL_COMPUTE_SHADER == 0 {
		t.Error("GL_COMPUTE_SHADER should be non-zero")
	}
	if GL_SHADER_STORAGE_BUFFER == 0 {
		t.Error("GL_SHADER_STORAGE_BUFFER should be non-zero")
	}
	if GL_ALL_BARRIER_BITS == 0 {
		t.Error("GL_ALL_BARRIER_BITS should be non-zero")
	}
}

func TestGLFullWorkflow(t *testing.T) {
	gl := newGL(t)

	// Create and compile shader
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.ShaderSource(shader, "layout(local_size_x=64) in; void main() {}")
	gl.CompileShader(shader)

	// Create and link program
	prog := gl.CreateProgram()
	gl.AttachShader(prog, shader)
	gl.LinkProgram(prog)
	gl.UseProgram(prog)

	// Create buffers
	bufs := gl.GenBuffers(2)
	for i, b := range bufs {
		gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, b)
		gl.BufferData(GL_SHADER_STORAGE_BUFFER, 256, nil, GL_DYNAMIC_DRAW)
		gl.BindBufferBase(GL_SHADER_STORAGE_BUFFER, i, b)
	}

	// Dispatch
	err := gl.DispatchCompute(4, 1, 1)
	if err != nil {
		t.Fatalf("DispatchCompute failed: %v", err)
	}

	// Barrier and sync
	gl.MemoryBarrier(GL_SHADER_STORAGE_BARRIER_BIT)
	sync := gl.FenceSync()
	result := gl.ClientWaitSync(sync, GL_SYNC_FLUSH_COMMANDS_BIT, 1000000)
	if result != GL_ALREADY_SIGNALED && result != GL_CONDITION_SATISFIED {
		t.Errorf("unexpected sync result: 0x%04X", result)
	}

	// Cleanup
	gl.DeleteSync(sync)
	gl.DeleteProgram(prog)
	gl.DeleteShader(shader)
	gl.DeleteBuffers(bufs)
}
