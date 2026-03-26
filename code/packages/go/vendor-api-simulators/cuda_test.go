package vendorapisimulators

import (
	"testing"
)

// =========================================================================
// CUDA Runtime tests
// =========================================================================

func newCUDA(t *testing.T) *CUDARuntime {
	t.Helper()
	cuda, err := NewCUDARuntime()
	if err != nil {
		t.Fatalf("NewCUDARuntime failed: %v", err)
	}
	return cuda
}

func TestCUDACreation(t *testing.T) {
	cuda := newCUDA(t)
	if cuda.PhysicalDevice.Vendor() != "nvidia" {
		t.Errorf("expected nvidia vendor, got %s", cuda.PhysicalDevice.Vendor())
	}
}

func TestCUDAGetSetDevice(t *testing.T) {
	cuda := newCUDA(t)
	if cuda.GetDevice() != 0 {
		t.Errorf("expected device 0, got %d", cuda.GetDevice())
	}
	// SetDevice to device 1 (should exist with default devices)
	err := cuda.SetDevice(1)
	if err != nil {
		t.Fatalf("SetDevice(1) failed: %v", err)
	}
	if cuda.GetDevice() != 1 {
		t.Errorf("expected device 1, got %d", cuda.GetDevice())
	}
}

func TestCUDASetDeviceInvalid(t *testing.T) {
	cuda := newCUDA(t)
	err := cuda.SetDevice(-1)
	if err == nil {
		t.Error("expected error for negative device ID")
	}
	err = cuda.SetDevice(9999)
	if err == nil {
		t.Error("expected error for out-of-range device ID")
	}
}

func TestCUDADeviceProperties(t *testing.T) {
	cuda := newCUDA(t)
	props := cuda.GetDeviceProperties()
	if props.Name == "" {
		t.Error("expected non-empty device name")
	}
	if props.TotalGlobalMem <= 0 {
		t.Error("expected positive total global memory")
	}
	if props.WarpSize != 32 {
		t.Errorf("expected warp size 32, got %d", props.WarpSize)
	}
	if props.MaxThreadsPerBlock <= 0 {
		t.Error("expected positive max threads per block")
	}
	if props.SharedMemPerBlock != 49152 {
		t.Errorf("expected 49152 shared mem, got %d", props.SharedMemPerBlock)
	}
	if props.ComputeCapability != [2]int{8, 0} {
		t.Errorf("unexpected compute capability: %v", props.ComputeCapability)
	}
}

func TestCUDAMalloc(t *testing.T) {
	cuda := newCUDA(t)
	ptr, err := cuda.Malloc(1024)
	if err != nil {
		t.Fatalf("Malloc failed: %v", err)
	}
	if ptr.Size != 1024 {
		t.Errorf("expected size 1024, got %d", ptr.Size)
	}
	// DeviceAddress may start at 0 for the first allocation.
	if ptr.Buffer == nil {
		t.Error("expected non-nil buffer")
	}
}

func TestCUDAMallocManaged(t *testing.T) {
	cuda := newCUDA(t)
	ptr, err := cuda.MallocManaged(512)
	if err != nil {
		t.Fatalf("MallocManaged failed: %v", err)
	}
	if ptr.Size != 512 {
		t.Errorf("expected size 512, got %d", ptr.Size)
	}
}

func TestCUDAFree(t *testing.T) {
	cuda := newCUDA(t)
	ptr, _ := cuda.Malloc(256)
	err := cuda.Free(ptr)
	if err != nil {
		t.Fatalf("Free failed: %v", err)
	}
	// Double free should error
	err = cuda.Free(ptr)
	if err == nil {
		t.Error("expected error for double free")
	}
}

func TestCUDAMemcpyHostToDevice(t *testing.T) {
	cuda := newCUDA(t)
	dst, _ := cuda.Malloc(8)
	src := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	err := cuda.Memcpy(dst, nil, src, nil, 8, CUDAMemcpyHostToDevice)
	if err != nil {
		t.Fatalf("Memcpy H2D failed: %v", err)
	}
}

func TestCUDAMemcpyDeviceToHost(t *testing.T) {
	cuda := newCUDA(t)
	devPtr, _ := cuda.Malloc(8)
	src := []byte{10, 20, 30, 40, 50, 60, 70, 80}
	cuda.Memcpy(devPtr, nil, src, nil, 8, CUDAMemcpyHostToDevice)

	dst := make([]byte, 8)
	err := cuda.Memcpy(nil, dst, nil, devPtr, 8, CUDAMemcpyDeviceToHost)
	if err != nil {
		t.Fatalf("Memcpy D2H failed: %v", err)
	}
	for i := 0; i < 8; i++ {
		if dst[i] != src[i] {
			t.Errorf("byte %d: expected %d, got %d", i, src[i], dst[i])
		}
	}
}

func TestCUDAMemcpyDeviceToDevice(t *testing.T) {
	cuda := newCUDA(t)
	src, _ := cuda.Malloc(8)
	dst, _ := cuda.Malloc(8)
	data := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	cuda.Memcpy(src, nil, data, nil, 8, CUDAMemcpyHostToDevice)
	err := cuda.Memcpy(dst, nil, nil, src, 8, CUDAMemcpyDeviceToDevice)
	if err != nil {
		t.Fatalf("Memcpy D2D failed: %v", err)
	}
}

func TestCUDAMemcpyHostToHost(t *testing.T) {
	cuda := newCUDA(t)
	src := []byte{5, 10, 15, 20}
	dst := make([]byte, 4)
	err := cuda.Memcpy(nil, dst, src, nil, 4, CUDAMemcpyHostToHost)
	if err != nil {
		t.Fatalf("Memcpy H2H failed: %v", err)
	}
	for i := 0; i < 4; i++ {
		if dst[i] != src[i] {
			t.Errorf("byte %d: expected %d, got %d", i, src[i], dst[i])
		}
	}
}

func TestCUDAMemcpyTypeErrors(t *testing.T) {
	cuda := newCUDA(t)
	// H2D without dst ptr
	err := cuda.Memcpy(nil, nil, []byte{1}, nil, 1, CUDAMemcpyHostToDevice)
	if err == nil {
		t.Error("expected error for nil dst in H2D")
	}
	// H2D without src data
	ptr, _ := cuda.Malloc(4)
	err = cuda.Memcpy(ptr, nil, nil, nil, 1, CUDAMemcpyHostToDevice)
	if err == nil {
		t.Error("expected error for nil src in H2D")
	}
	// D2H without src ptr
	err = cuda.Memcpy(nil, make([]byte, 4), nil, nil, 1, CUDAMemcpyDeviceToHost)
	if err == nil {
		t.Error("expected error for nil src in D2H")
	}
	// D2H without dst buf
	err = cuda.Memcpy(nil, nil, nil, ptr, 1, CUDAMemcpyDeviceToHost)
	if err == nil {
		t.Error("expected error for nil dst in D2H")
	}
}

func TestCUDAMemset(t *testing.T) {
	cuda := newCUDA(t)
	ptr, _ := cuda.Malloc(16)
	err := cuda.Memset(ptr, 0xFF, 16)
	if err != nil {
		t.Fatalf("Memset failed: %v", err)
	}
}

func TestCUDALaunchKernel(t *testing.T) {
	cuda := newCUDA(t)
	ptr, _ := cuda.Malloc(256)
	kernel := CUDAKernel{Name: "test_kernel"}
	grid := NewDim3(4, 1, 1)
	block := NewDim3(64, 1, 1)
	err := cuda.LaunchKernel(kernel, grid, block, []*CUDADevicePtr{ptr}, 0, nil)
	if err != nil {
		t.Fatalf("LaunchKernel failed: %v", err)
	}
}

func TestCUDALaunchKernelNoArgs(t *testing.T) {
	cuda := newCUDA(t)
	kernel := CUDAKernel{Name: "no_args_kernel"}
	err := cuda.LaunchKernel(kernel, NewDim3(1, 1, 1), NewDim3(32, 1, 1), nil, 0, nil)
	if err != nil {
		t.Fatalf("LaunchKernel with no args failed: %v", err)
	}
}

func TestCUDALaunchKernelMultipleArgs(t *testing.T) {
	cuda := newCUDA(t)
	a, _ := cuda.Malloc(64)
	b, _ := cuda.Malloc(64)
	c, _ := cuda.Malloc(64)
	kernel := CUDAKernel{Name: "multi_arg"}
	err := cuda.LaunchKernel(kernel, NewDim3(2, 1, 1), NewDim3(32, 1, 1),
		[]*CUDADevicePtr{a, b, c}, 0, nil)
	if err != nil {
		t.Fatalf("LaunchKernel with multiple args failed: %v", err)
	}
}

func TestCUDALaunchKernelOnStream(t *testing.T) {
	cuda := newCUDA(t)
	stream := cuda.CreateStream()
	ptr, _ := cuda.Malloc(64)
	kernel := CUDAKernel{Name: "stream_kernel"}
	err := cuda.LaunchKernel(kernel, NewDim3(1, 1, 1), NewDim3(32, 1, 1),
		[]*CUDADevicePtr{ptr}, 0, stream)
	if err != nil {
		t.Fatalf("LaunchKernel on stream failed: %v", err)
	}
}

func TestCUDACreateDestroyStream(t *testing.T) {
	cuda := newCUDA(t)
	s1 := cuda.CreateStream()
	s2 := cuda.CreateStream()
	if len(cuda.streams) != 2 {
		t.Errorf("expected 2 streams, got %d", len(cuda.streams))
	}
	err := cuda.DestroyStream(s1)
	if err != nil {
		t.Fatalf("DestroyStream failed: %v", err)
	}
	if len(cuda.streams) != 1 {
		t.Errorf("expected 1 stream after destroy, got %d", len(cuda.streams))
	}
	err = cuda.DestroyStream(s2)
	if err != nil {
		t.Fatalf("DestroyStream s2 failed: %v", err)
	}
}

func TestCUDADestroyStreamNotFound(t *testing.T) {
	cuda := newCUDA(t)
	err := cuda.DestroyStream(&CUDAStream{})
	if err == nil {
		t.Error("expected error for destroying non-existent stream")
	}
}

func TestCUDAStreamSynchronize(t *testing.T) {
	cuda := newCUDA(t)
	stream := cuda.CreateStream()
	// Should not panic even without pending fence
	cuda.StreamSynchronize(stream)
}

func TestCUDACreateEvent(t *testing.T) {
	cuda := newCUDA(t)
	event := cuda.CreateEvent()
	if event == nil {
		t.Fatal("expected non-nil event")
	}
	if event.recorded {
		t.Error("event should not be recorded initially")
	}
}

func TestCUDARecordAndSyncEvent(t *testing.T) {
	cuda := newCUDA(t)
	event := cuda.CreateEvent()
	cuda.RecordEvent(event, nil)
	if !event.recorded {
		t.Error("event should be recorded after RecordEvent")
	}
	err := cuda.SynchronizeEvent(event)
	if err != nil {
		t.Fatalf("SynchronizeEvent failed: %v", err)
	}
}

func TestCUDARecordEventOnStream(t *testing.T) {
	cuda := newCUDA(t)
	stream := cuda.CreateStream()
	event := cuda.CreateEvent()
	cuda.RecordEvent(event, stream)
	if !event.recorded {
		t.Error("event should be recorded")
	}
}

func TestCUDASyncEventNotRecorded(t *testing.T) {
	cuda := newCUDA(t)
	event := cuda.CreateEvent()
	err := cuda.SynchronizeEvent(event)
	if err == nil {
		t.Error("expected error for syncing unrecorded event")
	}
}

func TestCUDAElapsedTime(t *testing.T) {
	cuda := newCUDA(t)
	start := cuda.CreateEvent()
	end := cuda.CreateEvent()
	cuda.RecordEvent(start, nil)
	// Launch a kernel to advance cycles
	ptr, _ := cuda.Malloc(64)
	cuda.LaunchKernel(CUDAKernel{Name: "timing_test"}, NewDim3(1, 1, 1),
		NewDim3(32, 1, 1), []*CUDADevicePtr{ptr}, 0, nil)
	cuda.RecordEvent(end, nil)

	elapsed, err := cuda.ElapsedTime(start, end)
	if err != nil {
		t.Fatalf("ElapsedTime failed: %v", err)
	}
	if elapsed < 0 {
		t.Errorf("expected non-negative elapsed time, got %f", elapsed)
	}
}

func TestCUDAElapsedTimeErrors(t *testing.T) {
	cuda := newCUDA(t)
	e1 := cuda.CreateEvent()
	e2 := cuda.CreateEvent()
	cuda.RecordEvent(e2, nil)

	_, err := cuda.ElapsedTime(e1, e2)
	if err == nil {
		t.Error("expected error for unrecorded start event")
	}
	cuda.RecordEvent(e1, nil)
	e3 := cuda.CreateEvent()
	_, err = cuda.ElapsedTime(e1, e3)
	if err == nil {
		t.Error("expected error for unrecorded end event")
	}
}

func TestCUDADeviceSynchronize(t *testing.T) {
	cuda := newCUDA(t)
	// Should not panic
	cuda.DeviceSynchronize()
}

func TestCUDADeviceReset(t *testing.T) {
	cuda := newCUDA(t)
	cuda.CreateStream()
	cuda.CreateEvent()
	cuda.DeviceReset()
	if len(cuda.streams) != 0 {
		t.Error("expected streams to be cleared after reset")
	}
	if len(cuda.events) != 0 {
		t.Error("expected events to be cleared after reset")
	}
}

func TestCUDADeviceCount(t *testing.T) {
	cuda := newCUDA(t)
	count := cuda.DeviceCount()
	if count < 1 {
		t.Errorf("expected at least 1 device, got %d", count)
	}
}

func TestCUDADim3(t *testing.T) {
	d := NewDim3(4, 2, 1)
	if d.X != 4 || d.Y != 2 || d.Z != 1 {
		t.Errorf("unexpected Dim3: %v", d)
	}
}

func TestCUDAFullWorkflow(t *testing.T) {
	// Test a complete CUDA workflow: malloc, memcpy, launch, memcpy, free
	cuda := newCUDA(t)
	dX, _ := cuda.Malloc(32)
	dY, _ := cuda.Malloc(32)

	hostX := make([]byte, 32)
	for i := range hostX {
		hostX[i] = byte(i)
	}
	cuda.Memcpy(dX, nil, hostX, nil, 32, CUDAMemcpyHostToDevice)

	kernel := CUDAKernel{Name: "saxpy"}
	cuda.LaunchKernel(kernel, NewDim3(1, 1, 1), NewDim3(32, 1, 1),
		[]*CUDADevicePtr{dX, dY}, 0, nil)
	cuda.DeviceSynchronize()

	hostY := make([]byte, 32)
	cuda.Memcpy(nil, hostY, nil, dY, 32, CUDAMemcpyDeviceToHost)

	cuda.Free(dX)
	cuda.Free(dY)
}
