package vendorapisimulators

import (
	"testing"

	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// WebGPU tests
// =========================================================================

func newWebGPU(t *testing.T) *GPUDevice {
	t.Helper()
	dev, err := NewGPUDevice("")
	if err != nil {
		t.Fatalf("NewGPUDevice failed: %v", err)
	}
	return dev
}

func TestGPUDeviceCreation(t *testing.T) {
	dev := newWebGPU(t)
	if dev.Queue == nil {
		t.Fatal("expected non-nil Queue")
	}
	if dev.Features == nil {
		t.Fatal("expected non-nil Features")
	}
	if !dev.Features["compute"] {
		t.Error("expected 'compute' feature")
	}
}

func TestGPUDeviceLimits(t *testing.T) {
	dev := newWebGPU(t)
	if dev.Limits.MaxBufferSize <= 0 {
		t.Error("expected positive max buffer size")
	}
	if dev.Limits.MaxComputeWorkgroupSizeX <= 0 {
		t.Error("expected positive max compute workgroup size X")
	}
}

func TestGPUDeviceWithVendorHint(t *testing.T) {
	dev, err := NewGPUDevice("nvidia")
	if err != nil {
		t.Fatalf("NewGPUDevice with vendor hint failed: %v", err)
	}
	if dev.PhysicalDevice.Vendor() != "nvidia" {
		t.Errorf("expected nvidia vendor, got %s", dev.PhysicalDevice.Vendor())
	}
}

func TestGPUCreateBuffer(t *testing.T) {
	dev := newWebGPU(t)
	buf, err := dev.CreateBuffer(GPUBufferDescriptor{
		Size:  256,
		Usage: GPUBufferUsageStorage | GPUBufferUsageCopyDst,
	})
	if err != nil {
		t.Fatalf("CreateBuffer failed: %v", err)
	}
	if buf.Size() != 256 {
		t.Errorf("expected size 256, got %d", buf.Size())
	}
	if buf.Usage() != GPUBufferUsageStorage|GPUBufferUsageCopyDst {
		t.Errorf("unexpected usage flags: %d", buf.Usage())
	}
}

func TestGPUCreateBufferMappedAtCreation(t *testing.T) {
	dev := newWebGPU(t)
	buf, err := dev.CreateBuffer(GPUBufferDescriptor{
		Size:             64,
		Usage:            GPUBufferUsageStorage,
		MappedAtCreation: true,
	})
	if err != nil {
		t.Fatalf("CreateBuffer with MappedAtCreation failed: %v", err)
	}
	if !buf.mapped {
		t.Error("expected buffer to be mapped at creation")
	}
}

func TestGPUBufferMapAsyncAndGetMappedRange(t *testing.T) {
	dev := newWebGPU(t)
	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{
		Size:  32,
		Usage: GPUBufferUsageStorage | GPUBufferUsageMapRead,
	})

	// Write some data first
	dev.Queue.WriteBuffer(buf, 0, []byte{10, 20, 30, 40})

	err := buf.MapAsync(GPUMapModeRead, 0, 32)
	if err != nil {
		t.Fatalf("MapAsync failed: %v", err)
	}

	data, err := buf.GetMappedRange(0, 4)
	if err != nil {
		t.Fatalf("GetMappedRange failed: %v", err)
	}
	if len(data) < 4 {
		t.Fatal("expected at least 4 bytes")
	}
}

func TestGPUBufferGetMappedRangeNotMapped(t *testing.T) {
	dev := newWebGPU(t)
	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 16, Usage: GPUBufferUsageStorage})
	_, err := buf.GetMappedRange(0, 0)
	if err == nil {
		t.Error("expected error for unmapped buffer")
	}
}

func TestGPUBufferUnmap(t *testing.T) {
	dev := newWebGPU(t)
	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{
		Size:             32,
		Usage:            GPUBufferUsageStorage,
		MappedAtCreation: true,
	})
	err := buf.Unmap()
	if err != nil {
		t.Fatalf("Unmap failed: %v", err)
	}
	if buf.mapped {
		t.Error("buffer should not be mapped after Unmap")
	}
}

func TestGPUBufferUnmapNotMapped(t *testing.T) {
	dev := newWebGPU(t)
	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 16, Usage: GPUBufferUsageStorage})
	err := buf.Unmap()
	if err == nil {
		t.Error("expected error for unmapping buffer that is not mapped")
	}
}

func TestGPUBufferDestroy(t *testing.T) {
	dev := newWebGPU(t)
	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 64, Usage: GPUBufferUsageStorage})
	err := buf.Destroy()
	if err != nil {
		t.Fatalf("Destroy failed: %v", err)
	}
	if !buf.destroyed {
		t.Error("buffer should be destroyed")
	}
	// Double destroy should be no-op
	err = buf.Destroy()
	if err != nil {
		t.Fatalf("double Destroy should not error: %v", err)
	}
}

func TestGPUBufferMapAsyncDestroyed(t *testing.T) {
	dev := newWebGPU(t)
	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 32, Usage: GPUBufferUsageStorage})
	buf.Destroy()
	err := buf.MapAsync(GPUMapModeRead, 0, 0)
	if err == nil {
		t.Error("expected error for mapping destroyed buffer")
	}
}

func TestGPUCreateShaderModule(t *testing.T) {
	dev := newWebGPU(t)
	sm := dev.CreateShaderModule(GPUShaderModuleDescriptor{
		Code: []gpucore.Instruction{},
	})
	if sm == nil {
		t.Fatal("expected non-nil shader module")
	}
}

func TestGPUCreateShaderModuleNoCode(t *testing.T) {
	dev := newWebGPU(t)
	sm := dev.CreateShaderModule(GPUShaderModuleDescriptor{Code: "wgsl_source"})
	if sm == nil {
		t.Fatal("expected non-nil shader module even with string code")
	}
}

func TestGPUCreateComputePipeline(t *testing.T) {
	dev := newWebGPU(t)
	sm := dev.CreateShaderModule(GPUShaderModuleDescriptor{})
	pipeline := dev.CreateComputePipeline(GPUComputePipelineDescriptor{
		Layout: "auto",
		Compute: &GPUProgrammableStage{
			Module:     sm,
			EntryPoint: "main",
		},
	})
	if pipeline == nil {
		t.Fatal("expected non-nil compute pipeline")
	}
}

func TestGPUCreateComputePipelineNoShader(t *testing.T) {
	dev := newWebGPU(t)
	pipeline := dev.CreateComputePipeline(GPUComputePipelineDescriptor{
		Layout: "auto",
	})
	if pipeline == nil {
		t.Fatal("expected non-nil compute pipeline even without explicit shader")
	}
}

func TestGPUComputePipelineGetBindGroupLayout(t *testing.T) {
	dev := newWebGPU(t)
	pipeline := dev.CreateComputePipeline(GPUComputePipelineDescriptor{Layout: "auto"})
	bgl, err := pipeline.GetBindGroupLayout(0)
	if err != nil {
		t.Fatalf("GetBindGroupLayout failed: %v", err)
	}
	if bgl == nil {
		t.Fatal("expected non-nil bind group layout")
	}
}

func TestGPUComputePipelineGetBindGroupLayoutOutOfRange(t *testing.T) {
	dev := newWebGPU(t)
	pipeline := dev.CreateComputePipeline(GPUComputePipelineDescriptor{Layout: "auto"})
	_, err := pipeline.GetBindGroupLayout(99)
	if err == nil {
		t.Error("expected error for out-of-range index")
	}
}

func TestGPUCreateBindGroupLayout(t *testing.T) {
	dev := newWebGPU(t)
	bgl := dev.CreateBindGroupLayout(GPUBindGroupLayoutDescriptor{
		Entries: []GPUBindGroupLayoutEntry{
			{Binding: 0, Visibility: 4, BufferType: "storage"},
		},
	})
	if bgl == nil {
		t.Fatal("expected non-nil bind group layout")
	}
}

func TestGPUCreateBindGroupLayoutDefaultType(t *testing.T) {
	dev := newWebGPU(t)
	bgl := dev.CreateBindGroupLayout(GPUBindGroupLayoutDescriptor{
		Entries: []GPUBindGroupLayoutEntry{
			{Binding: 0, Visibility: 4, BufferType: ""}, // empty defaults to "storage"
		},
	})
	if bgl == nil {
		t.Fatal("expected non-nil bind group layout")
	}
}

func TestGPUCreatePipelineLayout(t *testing.T) {
	dev := newWebGPU(t)
	bgl := dev.CreateBindGroupLayout(GPUBindGroupLayoutDescriptor{
		Entries: []GPUBindGroupLayoutEntry{
			{Binding: 0, BufferType: "storage"},
		},
	})
	pl := dev.CreatePipelineLayout(GPUPipelineLayoutDescriptor{
		BindGroupLayouts: []*GPUBindGroupLayout{bgl},
	})
	if pl == nil {
		t.Fatal("expected non-nil pipeline layout")
	}
}

func TestGPUCreateBindGroup(t *testing.T) {
	dev := newWebGPU(t)
	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 64, Usage: GPUBufferUsageStorage})
	bgl := dev.CreateBindGroupLayout(GPUBindGroupLayoutDescriptor{
		Entries: []GPUBindGroupLayoutEntry{
			{Binding: 0, BufferType: "storage"},
		},
	})
	bg := dev.CreateBindGroup(GPUBindGroupDescriptor{
		Layout: bgl,
		Entries: []GPUBindGroupEntry{
			{Binding: 0, Resource: buf},
		},
	})
	if bg == nil {
		t.Fatal("expected non-nil bind group")
	}
}

func TestGPUCreateBindGroupNoLayout(t *testing.T) {
	dev := newWebGPU(t)
	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 32, Usage: GPUBufferUsageStorage})
	bg := dev.CreateBindGroup(GPUBindGroupDescriptor{
		Entries: []GPUBindGroupEntry{
			{Binding: 0, Resource: buf},
		},
	})
	if bg == nil {
		t.Fatal("expected non-nil bind group even without explicit layout")
	}
}

func TestGPUCreateCommandEncoder(t *testing.T) {
	dev := newWebGPU(t)
	enc, err := dev.CreateCommandEncoder()
	if err != nil {
		t.Fatalf("CreateCommandEncoder failed: %v", err)
	}
	if enc == nil {
		t.Fatal("expected non-nil command encoder")
	}
}

func TestGPUCommandEncoderFinish(t *testing.T) {
	dev := newWebGPU(t)
	enc, _ := dev.CreateCommandEncoder()
	cb, err := enc.Finish()
	if err != nil {
		t.Fatalf("Finish failed: %v", err)
	}
	if cb == nil {
		t.Fatal("expected non-nil command buffer")
	}
}

func TestGPUComputePassEncoder(t *testing.T) {
	dev := newWebGPU(t)
	enc, _ := dev.CreateCommandEncoder()
	pass := enc.BeginComputePass()
	if pass == nil {
		t.Fatal("expected non-nil compute pass encoder")
	}

	pipeline := dev.CreateComputePipeline(GPUComputePipelineDescriptor{Layout: "auto"})
	pass.SetPipeline(pipeline)

	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 64, Usage: GPUBufferUsageStorage})
	bg := dev.CreateBindGroup(GPUBindGroupDescriptor{
		Entries: []GPUBindGroupEntry{{Binding: 0, Resource: buf}},
	})
	pass.SetBindGroup(0, bg)

	err := pass.DispatchWorkgroups(4, 1, 1)
	if err != nil {
		t.Fatalf("DispatchWorkgroups failed: %v", err)
	}
	pass.End()
}

func TestGPUComputePassEncoderNoPipeline(t *testing.T) {
	dev := newWebGPU(t)
	enc, _ := dev.CreateCommandEncoder()
	pass := enc.BeginComputePass()
	err := pass.DispatchWorkgroups(1, 1, 1)
	if err == nil {
		t.Error("expected error for dispatch without pipeline")
	}
}

func TestGPUCopyBufferToBuffer(t *testing.T) {
	dev := newWebGPU(t)
	src, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 64, Usage: GPUBufferUsageCopySrc})
	dst, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 64, Usage: GPUBufferUsageCopyDst})
	enc, _ := dev.CreateCommandEncoder()
	err := enc.CopyBufferToBuffer(src, 0, dst, 0, 64)
	if err != nil {
		t.Fatalf("CopyBufferToBuffer failed: %v", err)
	}
}

func TestGPUQueueSubmit(t *testing.T) {
	dev := newWebGPU(t)
	enc, _ := dev.CreateCommandEncoder()
	cb, _ := enc.Finish()
	err := dev.Queue.Submit([]*GPUCommandBuffer{cb})
	if err != nil {
		t.Fatalf("Submit failed: %v", err)
	}
}

func TestGPUQueueWriteBuffer(t *testing.T) {
	dev := newWebGPU(t)
	buf, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 16, Usage: GPUBufferUsageStorage})
	err := dev.Queue.WriteBuffer(buf, 0, []byte{1, 2, 3, 4})
	if err != nil {
		t.Fatalf("WriteBuffer failed: %v", err)
	}
}

func TestGPUDestroyDevice(t *testing.T) {
	dev := newWebGPU(t)
	dev.DestroyDevice() // Should not panic
}

func TestGPUEntryPoint(t *testing.T) {
	gpu, err := NewGPU()
	if err != nil {
		t.Fatalf("NewGPU failed: %v", err)
	}
	if gpu == nil {
		t.Fatal("expected non-nil GPU")
	}
}

func TestGPURequestAdapter(t *testing.T) {
	gpu, _ := NewGPU()
	adapter, err := gpu.RequestAdapter(nil)
	if err != nil {
		t.Fatalf("RequestAdapter failed: %v", err)
	}
	if adapter.Name() == "" {
		t.Error("expected non-empty adapter name")
	}
	if !adapter.Features["compute"] {
		t.Error("expected compute feature")
	}
}

func TestGPURequestAdapterLowPower(t *testing.T) {
	gpu, _ := NewGPU()
	adapter, err := gpu.RequestAdapter(&GPURequestAdapterOptions{
		PowerPreference: "low-power",
	})
	if err != nil {
		t.Fatalf("RequestAdapter low-power failed: %v", err)
	}
	if adapter == nil {
		t.Fatal("expected non-nil adapter")
	}
}

func TestGPURequestAdapterHighPerformance(t *testing.T) {
	gpu, _ := NewGPU()
	adapter, err := gpu.RequestAdapter(&GPURequestAdapterOptions{
		PowerPreference: "high-performance",
	})
	if err != nil {
		t.Fatalf("RequestAdapter high-performance failed: %v", err)
	}
	if adapter == nil {
		t.Fatal("expected non-nil adapter")
	}
}

func TestGPUAdapterRequestDevice(t *testing.T) {
	gpu, _ := NewGPU()
	adapter, _ := gpu.RequestAdapter(nil)
	dev, err := adapter.RequestDevice()
	if err != nil {
		t.Fatalf("RequestDevice failed: %v", err)
	}
	if dev == nil {
		t.Fatal("expected non-nil device")
	}
	if dev.Queue == nil {
		t.Fatal("expected non-nil queue on device from adapter")
	}
}

func TestGPUAdapterLimits(t *testing.T) {
	gpu, _ := NewGPU()
	adapter, _ := gpu.RequestAdapter(nil)
	if adapter.Limits.MaxBufferSize <= 0 {
		t.Error("expected positive max buffer size on adapter")
	}
	if adapter.Limits.MaxComputeWorkgroupSizeX <= 0 {
		t.Error("expected positive max compute workgroup size X on adapter")
	}
}

func TestGPUBufferUsageFlags(t *testing.T) {
	combined := GPUBufferUsageStorage | GPUBufferUsageCopySrc | GPUBufferUsageCopyDst
	if combined&GPUBufferUsageStorage == 0 {
		t.Error("expected storage flag")
	}
	if combined&GPUBufferUsageCopySrc == 0 {
		t.Error("expected copy src flag")
	}
}

func TestGPUFullWorkflow(t *testing.T) {
	dev := newWebGPU(t)

	// Create buffers
	input, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 32, Usage: GPUBufferUsageStorage | GPUBufferUsageCopyDst})
	output, _ := dev.CreateBuffer(GPUBufferDescriptor{Size: 32, Usage: GPUBufferUsageStorage | GPUBufferUsageCopySrc})

	// Write data
	data := make([]byte, 32)
	for i := range data {
		data[i] = byte(i)
	}
	dev.Queue.WriteBuffer(input, 0, data)

	// Create pipeline
	sm := dev.CreateShaderModule(GPUShaderModuleDescriptor{Code: []gpucore.Instruction{}})
	pipeline := dev.CreateComputePipeline(GPUComputePipelineDescriptor{
		Layout:  "auto",
		Compute: &GPUProgrammableStage{Module: sm, EntryPoint: "main"},
	})

	// Create bind group
	bgl, _ := pipeline.GetBindGroupLayout(0)
	bg := dev.CreateBindGroup(GPUBindGroupDescriptor{
		Layout: bgl,
		Entries: []GPUBindGroupEntry{
			{Binding: 0, Resource: input},
		},
	})

	// Encode and submit
	enc, _ := dev.CreateCommandEncoder()
	pass := enc.BeginComputePass()
	pass.SetPipeline(pipeline)
	pass.SetBindGroup(0, bg)
	pass.DispatchWorkgroups(1, 1, 1)
	pass.End()

	enc.CopyBufferToBuffer(input, 0, output, 0, 32)
	cb, _ := enc.Finish()
	dev.Queue.Submit([]*GPUCommandBuffer{cb})

	dev.DestroyDevice()
}
