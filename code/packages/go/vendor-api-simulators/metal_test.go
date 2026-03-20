package vendorapisimulators

import (
	"testing"
)

// =========================================================================
// Metal tests
// =========================================================================

func newMetal(t *testing.T) *MTLDevice {
	t.Helper()
	dev, err := NewMTLDevice()
	if err != nil {
		t.Fatalf("NewMTLDevice failed: %v", err)
	}
	return dev
}

func TestMetalDeviceCreation(t *testing.T) {
	dev := newMetal(t)
	if dev.PhysicalDevice.Vendor() != "apple" {
		t.Errorf("expected apple vendor, got %s", dev.PhysicalDevice.Vendor())
	}
}

func TestMetalDeviceName(t *testing.T) {
	dev := newMetal(t)
	if dev.Name() == "" {
		t.Error("expected non-empty device name")
	}
}

func TestMetalMakeBuffer(t *testing.T) {
	dev := newMetal(t)
	buf, err := dev.MakeBuffer(1024, MTLResourceStorageModeShared)
	if err != nil {
		t.Fatalf("MakeBuffer failed: %v", err)
	}
	if buf.Length() != 1024 {
		t.Errorf("expected length 1024, got %d", buf.Length())
	}
}

func TestMetalBufferWriteAndContents(t *testing.T) {
	dev := newMetal(t)
	buf, _ := dev.MakeBuffer(8, MTLResourceStorageModeShared)

	data := []byte{10, 20, 30, 40, 50, 60, 70, 80}
	err := buf.WriteBytes(data, 0)
	if err != nil {
		t.Fatalf("WriteBytes failed: %v", err)
	}

	contents, err := buf.Contents()
	if err != nil {
		t.Fatalf("Contents failed: %v", err)
	}
	for i := 0; i < 8; i++ {
		if contents[i] != data[i] {
			t.Errorf("byte %d: expected %d, got %d", i, data[i], contents[i])
		}
	}
}

func TestMetalBufferWriteWithOffset(t *testing.T) {
	dev := newMetal(t)
	buf, _ := dev.MakeBuffer(16, MTLResourceStorageModeShared)
	err := buf.WriteBytes([]byte{0xAA, 0xBB}, 4)
	if err != nil {
		t.Fatalf("WriteBytes with offset failed: %v", err)
	}
}

func TestMetalMakeLibrary(t *testing.T) {
	dev := newMetal(t)
	lib := dev.MakeLibrary("my_shader_source")
	if lib == nil {
		t.Fatal("expected non-nil library")
	}
}

func TestMetalMakeFunction(t *testing.T) {
	dev := newMetal(t)
	lib := dev.MakeLibrary("source")
	fn := lib.MakeFunction("compute_fn")
	if fn.Name() != "compute_fn" {
		t.Errorf("expected function name 'compute_fn', got '%s'", fn.Name())
	}
}

func TestMetalMakeComputePipelineState(t *testing.T) {
	dev := newMetal(t)
	lib := dev.MakeLibrary("source")
	fn := lib.MakeFunction("test")
	pso := dev.MakeComputePipelineState(fn)
	if pso == nil {
		t.Fatal("expected non-nil pipeline state")
	}
	if pso.MaxTotalThreadsPerThreadgroup() != 1024 {
		t.Errorf("expected max threads 1024, got %d", pso.MaxTotalThreadsPerThreadgroup())
	}
}

func TestMetalCommandQueue(t *testing.T) {
	dev := newMetal(t)
	queue := dev.MakeCommandQueue()
	if queue == nil {
		t.Fatal("expected non-nil command queue")
	}
}

func TestMetalCommandBufferLifecycle(t *testing.T) {
	dev := newMetal(t)
	queue := dev.MakeCommandQueue()
	cb, err := queue.MakeCommandBuffer()
	if err != nil {
		t.Fatalf("MakeCommandBuffer failed: %v", err)
	}
	if cb.Status() != MTLCommandBufferStatusNotEnqueued {
		t.Error("expected notEnqueued status")
	}
	err = cb.Commit()
	if err != nil {
		t.Fatalf("Commit failed: %v", err)
	}
	if cb.Status() != MTLCommandBufferStatusCompleted {
		t.Errorf("expected completed status, got %d", cb.Status())
	}
	cb.WaitUntilCompleted()
}

func TestMetalComputeEncoder(t *testing.T) {
	dev := newMetal(t)
	lib := dev.MakeLibrary("source")
	fn := lib.MakeFunction("compute_fn")
	pso := dev.MakeComputePipelineState(fn)
	buf, _ := dev.MakeBuffer(256, MTLResourceStorageModeShared)

	queue := dev.MakeCommandQueue()
	cb, _ := queue.MakeCommandBuffer()
	encoder := cb.MakeComputeCommandEncoder()
	encoder.SetComputePipelineState(pso)
	encoder.SetBuffer(buf, 0, 0)
	err := encoder.DispatchThreadgroups(
		NewMTLSize(4, 1, 1),
		NewMTLSize(64, 1, 1),
	)
	if err != nil {
		t.Fatalf("DispatchThreadgroups failed: %v", err)
	}
	encoder.EndEncoding()
	cb.Commit()
	cb.WaitUntilCompleted()
}

func TestMetalComputeEncoderNoPipeline(t *testing.T) {
	dev := newMetal(t)
	queue := dev.MakeCommandQueue()
	cb, _ := queue.MakeCommandBuffer()
	encoder := cb.MakeComputeCommandEncoder()
	err := encoder.DispatchThreadgroups(NewMTLSize(1, 1, 1), NewMTLSize(32, 1, 1))
	if err == nil {
		t.Error("expected error for dispatch without pipeline")
	}
}

func TestMetalComputeEncoderMultipleBuffers(t *testing.T) {
	dev := newMetal(t)
	lib := dev.MakeLibrary("source")
	fn := lib.MakeFunction("fn")
	pso := dev.MakeComputePipelineState(fn)
	buf0, _ := dev.MakeBuffer(64, MTLResourceStorageModeShared)
	buf1, _ := dev.MakeBuffer(64, MTLResourceStorageModeShared)

	queue := dev.MakeCommandQueue()
	cb, _ := queue.MakeCommandBuffer()
	encoder := cb.MakeComputeCommandEncoder()
	encoder.SetComputePipelineState(pso)
	encoder.SetBuffer(buf0, 0, 0)
	encoder.SetBuffer(buf1, 0, 1)
	err := encoder.DispatchThreadgroups(NewMTLSize(2, 1, 1), NewMTLSize(32, 1, 1))
	if err != nil {
		t.Fatalf("DispatchThreadgroups with multiple buffers failed: %v", err)
	}
	encoder.EndEncoding()
	cb.Commit()
}

func TestMetalDispatchThreads(t *testing.T) {
	dev := newMetal(t)
	lib := dev.MakeLibrary("source")
	fn := lib.MakeFunction("fn")
	pso := dev.MakeComputePipelineState(fn)
	buf, _ := dev.MakeBuffer(128, MTLResourceStorageModeShared)

	queue := dev.MakeCommandQueue()
	cb, _ := queue.MakeCommandBuffer()
	encoder := cb.MakeComputeCommandEncoder()
	encoder.SetComputePipelineState(pso)
	encoder.SetBuffer(buf, 0, 0)
	err := encoder.DispatchThreads(
		NewMTLSize(256, 1, 1),
		NewMTLSize(64, 1, 1),
	)
	if err != nil {
		t.Fatalf("DispatchThreads failed: %v", err)
	}
	encoder.EndEncoding()
	cb.Commit()
}

func TestMetalBlitEncoder(t *testing.T) {
	dev := newMetal(t)
	src, _ := dev.MakeBuffer(64, MTLResourceStorageModeShared)
	dst, _ := dev.MakeBuffer(64, MTLResourceStorageModeShared)

	queue := dev.MakeCommandQueue()
	cb, _ := queue.MakeCommandBuffer()
	blit := cb.MakeBlitCommandEncoder()
	err := blit.CopyFromBuffer(src, 0, dst, 0, 64)
	if err != nil {
		t.Fatalf("CopyFromBuffer failed: %v", err)
	}
	blit.EndEncoding()
	cb.Commit()
}

func TestMetalBlitEncoderFill(t *testing.T) {
	dev := newMetal(t)
	buf, _ := dev.MakeBuffer(32, MTLResourceStorageModeShared)

	queue := dev.MakeCommandQueue()
	cb, _ := queue.MakeCommandBuffer()
	blit := cb.MakeBlitCommandEncoder()
	err := blit.FillBuffer(buf, 0, 32, 0xFF)
	if err != nil {
		t.Fatalf("FillBuffer failed: %v", err)
	}
	blit.EndEncoding()
	cb.Commit()
}

func TestMetalSetBytes(t *testing.T) {
	dev := newMetal(t)
	queue := dev.MakeCommandQueue()
	cb, _ := queue.MakeCommandBuffer()
	encoder := cb.MakeComputeCommandEncoder()
	encoder.SetBytes([]byte{1, 2, 3, 4}, 0)
	if len(encoder.pushData) != 1 {
		t.Errorf("expected 1 push data entry, got %d", len(encoder.pushData))
	}
	encoder.EndEncoding()
}

func TestMTLSize(t *testing.T) {
	s := NewMTLSize(4, 2, 1)
	if s.Width != 4 || s.Height != 2 || s.Depth != 1 {
		t.Errorf("unexpected MTLSize: %v", s)
	}
}

func TestMetalResourceOptions(t *testing.T) {
	dev := newMetal(t)
	// Test creating buffers with different options (all map to same underlying type)
	_, err := dev.MakeBuffer(64, MTLResourceStorageModePrivate)
	if err != nil {
		t.Fatalf("MakeBuffer with private mode failed: %v", err)
	}
	_, err = dev.MakeBuffer(64, MTLResourceStorageModeManaged)
	if err != nil {
		t.Fatalf("MakeBuffer with managed mode failed: %v", err)
	}
}

func TestMetalFullWorkflow(t *testing.T) {
	dev := newMetal(t)
	queue := dev.MakeCommandQueue()
	buf, _ := dev.MakeBuffer(32, MTLResourceStorageModeShared)

	// Write data
	data := make([]byte, 32)
	for i := range data {
		data[i] = byte(i * 2)
	}
	buf.WriteBytes(data, 0)

	// Create pipeline
	lib := dev.MakeLibrary("test_source")
	fn := lib.MakeFunction("test_fn")
	pso := dev.MakeComputePipelineState(fn)

	// Dispatch
	cb, _ := queue.MakeCommandBuffer()
	encoder := cb.MakeComputeCommandEncoder()
	encoder.SetComputePipelineState(pso)
	encoder.SetBuffer(buf, 0, 0)
	encoder.DispatchThreadgroups(NewMTLSize(1, 1, 1), NewMTLSize(32, 1, 1))
	encoder.EndEncoding()
	cb.Commit()
	cb.WaitUntilCompleted()

	// Read result
	contents, _ := buf.Contents()
	if len(contents) < 32 {
		t.Fatal("expected at least 32 bytes of content")
	}
}
