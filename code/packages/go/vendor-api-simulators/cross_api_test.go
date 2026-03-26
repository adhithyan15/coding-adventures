package vendorapisimulators

import (
	"testing"
)

// =========================================================================
// Cross-API tests -- verify that all six simulators can coexist and that
// shared Layer 5 primitives work correctly across different API wrappers.
// =========================================================================

func TestAllSimulatorsCanBeCreated(t *testing.T) {
	// Every simulator should successfully initialize.
	_, err := NewCUDARuntime()
	if err != nil {
		t.Fatalf("CUDA failed: %v", err)
	}
	_, err = NewCLContext(nil)
	if err != nil {
		t.Fatalf("OpenCL failed: %v", err)
	}
	_, err = NewMTLDevice()
	if err != nil {
		t.Fatalf("Metal failed: %v", err)
	}
	_, err = NewVkInstance()
	if err != nil {
		t.Fatalf("Vulkan failed: %v", err)
	}
	_, err = NewGPUDevice("")
	if err != nil {
		t.Fatalf("WebGPU failed: %v", err)
	}
	_, err = NewGLContext()
	if err != nil {
		t.Fatalf("OpenGL failed: %v", err)
	}
}

func TestAllSimulatorsHavePhysicalDevice(t *testing.T) {
	cuda, _ := NewCUDARuntime()
	if cuda.PhysicalDevice == nil {
		t.Error("CUDA: no physical device")
	}

	cl, _ := NewCLContext(nil)
	if cl.PhysicalDevice == nil {
		t.Error("OpenCL: no physical device")
	}

	mtl, _ := NewMTLDevice()
	if mtl.PhysicalDevice == nil {
		t.Error("Metal: no physical device")
	}

	vk, _ := NewVkInstance()
	if vk.PhysicalDevice == nil {
		t.Error("Vulkan: no physical device")
	}

	wgpu, _ := NewGPUDevice("")
	if wgpu.PhysicalDevice == nil {
		t.Error("WebGPU: no physical device")
	}

	gl, _ := NewGLContext()
	if gl.PhysicalDevice == nil {
		t.Error("OpenGL: no physical device")
	}
}

func TestAllSimulatorsHaveMemoryManager(t *testing.T) {
	cuda, _ := NewCUDARuntime()
	if cuda.MemoryManager == nil {
		t.Error("CUDA: no memory manager")
	}
	cl, _ := NewCLContext(nil)
	if cl.MemoryManager == nil {
		t.Error("OpenCL: no memory manager")
	}
	mtl, _ := NewMTLDevice()
	if mtl.MemoryManager == nil {
		t.Error("Metal: no memory manager")
	}
	vk, _ := NewVkInstance()
	if vk.MemoryManager == nil {
		t.Error("Vulkan: no memory manager")
	}
	wgpu, _ := NewGPUDevice("")
	if wgpu.MemoryManager == nil {
		t.Error("WebGPU: no memory manager")
	}
	gl, _ := NewGLContext()
	if gl.MemoryManager == nil {
		t.Error("OpenGL: no memory manager")
	}
}

func TestAllSimulatorsHaveComputeQueue(t *testing.T) {
	cuda, _ := NewCUDARuntime()
	if cuda.ComputeQueue == nil {
		t.Error("CUDA: no compute queue")
	}
	cl, _ := NewCLContext(nil)
	if cl.ComputeQueue == nil {
		t.Error("OpenCL: no compute queue")
	}
	mtl, _ := NewMTLDevice()
	if mtl.ComputeQueue == nil {
		t.Error("Metal: no compute queue")
	}
	vk, _ := NewVkInstance()
	if vk.ComputeQueue == nil {
		t.Error("Vulkan: no compute queue")
	}
	wgpu, _ := NewGPUDevice("")
	if wgpu.ComputeQueue == nil {
		t.Error("WebGPU: no compute queue")
	}
	gl, _ := NewGLContext()
	if gl.ComputeQueue == nil {
		t.Error("OpenGL: no compute queue")
	}
}

func TestVendorHintConsistency(t *testing.T) {
	// CUDA should always select an nvidia device.
	cuda, _ := NewCUDARuntime()
	if cuda.PhysicalDevice.Vendor() != "nvidia" {
		t.Errorf("CUDA vendor: expected nvidia, got %s", cuda.PhysicalDevice.Vendor())
	}

	// Metal should always select an apple device.
	mtl, _ := NewMTLDevice()
	if mtl.PhysicalDevice.Vendor() != "apple" {
		t.Errorf("Metal vendor: expected apple, got %s", mtl.PhysicalDevice.Vendor())
	}
}

func TestMultipleSimulatorsIndependent(t *testing.T) {
	// Creating multiple simulators should not interfere with each other.
	cuda1, _ := NewCUDARuntime()
	cuda2, _ := NewCUDARuntime()

	// Allocate on cuda1 -- should not affect cuda2.
	ptr, _ := cuda1.Malloc(64)
	cuda1.Free(ptr)

	// cuda2 should still work fine.
	ptr2, err := cuda2.Malloc(128)
	if err != nil {
		t.Fatalf("cuda2 Malloc failed after cuda1 free: %v", err)
	}
	if ptr2.Size != 128 {
		t.Errorf("expected 128, got %d", ptr2.Size)
	}
}

func TestCrossAPIBufferSizes(t *testing.T) {
	// Allocate a buffer through each API and verify size consistency.
	cuda, _ := NewCUDARuntime()
	cudaBuf, _ := cuda.Malloc(256)
	if cudaBuf.Size != 256 {
		t.Errorf("CUDA buffer size: expected 256, got %d", cudaBuf.Size)
	}

	cl, _ := NewCLContext(nil)
	clBuf, _ := cl.CreateBuffer(CLMemReadWrite, 256, nil)
	if clBuf.Size() != 256 {
		t.Errorf("OpenCL buffer size: expected 256, got %d", clBuf.Size())
	}

	mtl, _ := NewMTLDevice()
	mtlBuf, _ := mtl.MakeBuffer(256, MTLResourceStorageModeShared)
	if mtlBuf.Length() != 256 {
		t.Errorf("Metal buffer length: expected 256, got %d", mtlBuf.Length())
	}

	wgpu, _ := NewGPUDevice("")
	wgpuBuf, _ := wgpu.CreateBuffer(GPUBufferDescriptor{Size: 256, Usage: GPUBufferUsageStorage})
	if wgpuBuf.Size() != 256 {
		t.Errorf("WebGPU buffer size: expected 256, got %d", wgpuBuf.Size())
	}
}

func TestCrossAPIDispatchSmoke(t *testing.T) {
	// Each API should be able to dispatch a minimal compute workload.

	// CUDA
	cuda, _ := NewCUDARuntime()
	buf, _ := cuda.Malloc(64)
	err := cuda.LaunchKernel(
		CUDAKernel{Name: "test"},
		NewDim3(1, 1, 1), NewDim3(32, 1, 1),
		[]*CUDADevicePtr{buf}, 0, nil,
	)
	if err != nil {
		t.Errorf("CUDA dispatch failed: %v", err)
	}

	// OpenCL
	cl, _ := NewCLContext(nil)
	prog := cl.CreateProgramWithSource("src")
	prog.Build(nil, "")
	kernel, _ := prog.CreateKernel("test")
	clBuf, _ := cl.CreateBuffer(CLMemReadWrite, 64, nil)
	kernel.SetArg(0, clBuf)
	queue := cl.CreateCommandQueue(nil)
	_, err = queue.EnqueueNDRangeKernel(kernel, []int{32}, []int{32}, nil)
	if err != nil {
		t.Errorf("OpenCL dispatch failed: %v", err)
	}

	// Metal
	mtl, _ := NewMTLDevice()
	lib := mtl.MakeLibrary("src")
	fn := lib.MakeFunction("test")
	pso := mtl.MakeComputePipelineState(fn)
	mtlBuf, _ := mtl.MakeBuffer(64, MTLResourceStorageModeShared)
	mq := mtl.MakeCommandQueue()
	cb, _ := mq.MakeCommandBuffer()
	enc := cb.MakeComputeCommandEncoder()
	enc.SetComputePipelineState(pso)
	enc.SetBuffer(mtlBuf, 0, 0)
	enc.DispatchThreadgroups(NewMTLSize(1, 1, 1), NewMTLSize(32, 1, 1))
	enc.EndEncoding()
	cb.Commit()

	// WebGPU
	wgpu, _ := NewGPUDevice("")
	wenc, _ := wgpu.CreateCommandEncoder()
	pass := wenc.BeginComputePass()
	pipeline := wgpu.CreateComputePipeline(GPUComputePipelineDescriptor{Layout: "auto"})
	pass.SetPipeline(pipeline)
	pass.DispatchWorkgroups(1, 1, 1)
	pass.End()
	wcb, _ := wenc.Finish()
	err = wgpu.Queue.Submit([]*GPUCommandBuffer{wcb})
	if err != nil {
		t.Errorf("WebGPU dispatch failed: %v", err)
	}

	// OpenGL
	gl, _ := NewGLContext()
	shader, _ := gl.CreateShader(GL_COMPUTE_SHADER)
	gl.CompileShader(shader)
	glProg := gl.CreateProgram()
	gl.AttachShader(glProg, shader)
	gl.LinkProgram(glProg)
	gl.UseProgram(glProg)
	glBufs := gl.GenBuffers(1)
	gl.BindBuffer(GL_SHADER_STORAGE_BUFFER, glBufs[0])
	gl.BufferData(GL_SHADER_STORAGE_BUFFER, 64, nil, GL_DYNAMIC_DRAW)
	gl.BindBufferBase(GL_SHADER_STORAGE_BUFFER, 0, glBufs[0])
	err = gl.DispatchCompute(1, 1, 1)
	if err != nil {
		t.Errorf("OpenGL dispatch failed: %v", err)
	}
}
