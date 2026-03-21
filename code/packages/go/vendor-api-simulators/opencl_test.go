package vendorapisimulators

import (
	"testing"
)

// =========================================================================
// OpenCL tests
// =========================================================================

func newCLContext(t *testing.T) *CLContext {
	t.Helper()
	ctx, err := NewCLContext(nil)
	if err != nil {
		t.Fatalf("NewCLContext failed: %v", err)
	}
	return ctx
}

func TestCLContextCreation(t *testing.T) {
	ctx := newCLContext(t)
	if ctx.LogicalDevice == nil {
		t.Error("expected non-nil LogicalDevice")
	}
	if len(ctx.devices) == 0 {
		t.Error("expected at least one device in context")
	}
}

func TestCLContextWithDevices(t *testing.T) {
	platform, err := NewCLPlatform()
	if err != nil {
		t.Fatalf("NewCLPlatform failed: %v", err)
	}
	devices := platform.GetDevices(CLDeviceTypeAll)
	if len(devices) == 0 {
		t.Fatal("expected at least one device")
	}
	ctx, err := NewCLContext(devices[:1])
	if err != nil {
		t.Fatalf("NewCLContext with devices failed: %v", err)
	}
	if len(ctx.Devices()) != 1 {
		t.Errorf("expected 1 device, got %d", len(ctx.Devices()))
	}
}

func TestCLDeviceProperties(t *testing.T) {
	platform, _ := NewCLPlatform()
	devices := platform.GetDevices(CLDeviceTypeAll)
	if len(devices) == 0 {
		t.Fatal("no devices")
	}
	dev := devices[0]
	if dev.Name() == "" {
		t.Error("expected non-empty name")
	}
	if dev.MaxWorkGroupSize() <= 0 {
		t.Error("expected positive max work group size")
	}
	if dev.GlobalMemSize() <= 0 {
		t.Error("expected positive global mem size")
	}
	if dev.MaxComputeUnits() <= 0 {
		t.Error("expected positive compute units")
	}
}

func TestCLDeviceGetInfo(t *testing.T) {
	platform, _ := NewCLPlatform()
	devices := platform.GetDevices(CLDeviceTypeAll)
	dev := devices[0]

	name := dev.GetInfo(CLDeviceInfoName)
	if name == nil || name.(string) == "" {
		t.Error("expected non-empty name from GetInfo")
	}
	dt := dev.GetInfo(CLDeviceInfoType)
	if dt == nil {
		t.Error("expected non-nil device type from GetInfo")
	}
	cu := dev.GetInfo(CLDeviceInfoMaxComputeUnits)
	if cu == nil {
		t.Error("expected non-nil compute units")
	}
	wg := dev.GetInfo(CLDeviceInfoMaxWorkGroupSize)
	if wg == nil {
		t.Error("expected non-nil work group size")
	}
	mem := dev.GetInfo(CLDeviceInfoGlobalMemSize)
	if mem == nil {
		t.Error("expected non-nil global mem size")
	}
}

func TestCLDeviceTypeMapping(t *testing.T) {
	platform, _ := NewCLPlatform()
	gpuDevices := platform.GetDevices(CLDeviceTypeGPU)
	if len(gpuDevices) == 0 {
		t.Error("expected at least one GPU device")
	}
}

func TestCLCreateBuffer(t *testing.T) {
	ctx := newCLContext(t)
	buf, err := ctx.CreateBuffer(CLMemReadWrite, 256, nil)
	if err != nil {
		t.Fatalf("CreateBuffer failed: %v", err)
	}
	if buf.Size() != 256 {
		t.Errorf("expected size 256, got %d", buf.Size())
	}
	if buf.Flags() != CLMemReadWrite {
		t.Errorf("expected ReadWrite flags, got %d", buf.Flags())
	}
}

func TestCLCreateBufferWithData(t *testing.T) {
	ctx := newCLContext(t)
	data := []byte{1, 2, 3, 4}
	buf, err := ctx.CreateBuffer(CLMemReadWrite|CLMemCopyHostPtr, 4, data)
	if err != nil {
		t.Fatalf("CreateBuffer with data failed: %v", err)
	}
	if buf.Size() != 4 {
		t.Errorf("expected size 4, got %d", buf.Size())
	}
}

func TestCLCreateBufferWithoutCopyFlag(t *testing.T) {
	ctx := newCLContext(t)
	data := []byte{1, 2, 3, 4}
	// Data provided but no COPY_HOST_PTR flag -- should not copy
	buf, err := ctx.CreateBuffer(CLMemReadWrite, 4, data)
	if err != nil {
		t.Fatalf("CreateBuffer failed: %v", err)
	}
	if buf.Size() != 4 {
		t.Errorf("expected size 4, got %d", buf.Size())
	}
}

func TestCLProgramBuildAndCreateKernel(t *testing.T) {
	ctx := newCLContext(t)
	prog := ctx.CreateProgramWithSource("my_kernel_source")
	if prog.BuildStatus() != CLBuildNone {
		t.Error("expected CLBuildNone before build")
	}
	prog.Build(nil, "")
	if prog.BuildStatus() != CLBuildSuccess {
		t.Error("expected CLBuildSuccess after build")
	}
	kernel, err := prog.CreateKernel("my_kernel")
	if err != nil {
		t.Fatalf("CreateKernel failed: %v", err)
	}
	if kernel.Name() != "my_kernel" {
		t.Errorf("expected kernel name 'my_kernel', got '%s'", kernel.Name())
	}
}

func TestCLCreateKernelUnbuilt(t *testing.T) {
	ctx := newCLContext(t)
	prog := ctx.CreateProgramWithSource("src")
	_, err := prog.CreateKernel("test")
	if err == nil {
		t.Error("expected error for creating kernel from unbuilt program")
	}
}

func TestCLKernelSetArg(t *testing.T) {
	ctx := newCLContext(t)
	prog := ctx.CreateProgramWithSource("src")
	prog.Build(nil, "")
	kernel, _ := prog.CreateKernel("test")
	buf, _ := ctx.CreateBuffer(CLMemReadWrite, 64, nil)
	kernel.SetArg(0, buf)
	kernel.SetArg(1, 42)
	if len(kernel.args) != 2 {
		t.Errorf("expected 2 args, got %d", len(kernel.args))
	}
}

func TestCLCommandQueueCreation(t *testing.T) {
	ctx := newCLContext(t)
	queue := ctx.CreateCommandQueue(nil)
	if queue == nil {
		t.Fatal("expected non-nil queue")
	}
}

func TestCLEnqueueWriteReadBuffer(t *testing.T) {
	ctx := newCLContext(t)
	buf, _ := ctx.CreateBuffer(CLMemReadWrite, 8, nil)
	queue := ctx.CreateCommandQueue(nil)

	data := []byte{10, 20, 30, 40, 50, 60, 70, 80}
	ev, err := queue.EnqueueWriteBuffer(buf, 0, 8, data, nil)
	if err != nil {
		t.Fatalf("EnqueueWriteBuffer failed: %v", err)
	}
	if ev.Status() != CLEventComplete {
		t.Error("expected write event to be complete")
	}

	result := make([]byte, 8)
	ev2, err := queue.EnqueueReadBuffer(buf, 0, 8, result, nil)
	if err != nil {
		t.Fatalf("EnqueueReadBuffer failed: %v", err)
	}
	if ev2.Status() != CLEventComplete {
		t.Error("expected read event to be complete")
	}
	for i := 0; i < 8; i++ {
		if result[i] != data[i] {
			t.Errorf("byte %d: expected %d, got %d", i, data[i], result[i])
		}
	}
}

func TestCLEnqueueWithWaitList(t *testing.T) {
	ctx := newCLContext(t)
	buf, _ := ctx.CreateBuffer(CLMemReadWrite, 4, nil)
	queue := ctx.CreateCommandQueue(nil)

	ev1, _ := queue.EnqueueWriteBuffer(buf, 0, 4, []byte{1, 2, 3, 4}, nil)
	result := make([]byte, 4)
	_, err := queue.EnqueueReadBuffer(buf, 0, 4, result, []*CLEvent{ev1})
	if err != nil {
		t.Fatalf("EnqueueReadBuffer with wait list failed: %v", err)
	}
}

func TestCLEnqueueCopyBuffer(t *testing.T) {
	ctx := newCLContext(t)
	src, _ := ctx.CreateBuffer(CLMemReadWrite, 8, nil)
	dst, _ := ctx.CreateBuffer(CLMemReadWrite, 8, nil)
	queue := ctx.CreateCommandQueue(nil)

	queue.EnqueueWriteBuffer(src, 0, 8, []byte{1, 2, 3, 4, 5, 6, 7, 8}, nil)
	ev, err := queue.EnqueueCopyBuffer(src, dst, 8, nil)
	if err != nil {
		t.Fatalf("EnqueueCopyBuffer failed: %v", err)
	}
	if ev == nil {
		t.Fatal("expected non-nil event")
	}
}

func TestCLEnqueueFillBuffer(t *testing.T) {
	ctx := newCLContext(t)
	buf, _ := ctx.CreateBuffer(CLMemReadWrite, 16, nil)
	queue := ctx.CreateCommandQueue(nil)

	ev, err := queue.EnqueueFillBuffer(buf, []byte{0xAA}, 0, 16)
	if err != nil {
		t.Fatalf("EnqueueFillBuffer failed: %v", err)
	}
	if ev == nil {
		t.Fatal("expected non-nil event")
	}
}

func TestCLEnqueueNDRangeKernel(t *testing.T) {
	ctx := newCLContext(t)
	prog := ctx.CreateProgramWithSource("kernel_src")
	prog.Build(nil, "")
	kernel, _ := prog.CreateKernel("my_kernel")
	buf, _ := ctx.CreateBuffer(CLMemReadWrite, 64, nil)
	kernel.SetArg(0, buf)

	queue := ctx.CreateCommandQueue(nil)
	ev, err := queue.EnqueueNDRangeKernel(kernel, []int{128}, []int{32}, nil)
	if err != nil {
		t.Fatalf("EnqueueNDRangeKernel failed: %v", err)
	}
	if ev == nil {
		t.Fatal("expected non-nil event")
	}
}

func TestCLEnqueueNDRangeKernelAutoLocalSize(t *testing.T) {
	ctx := newCLContext(t)
	prog := ctx.CreateProgramWithSource("src")
	prog.Build(nil, "")
	kernel, _ := prog.CreateKernel("k")
	buf, _ := ctx.CreateBuffer(CLMemReadWrite, 64, nil)
	kernel.SetArg(0, buf)

	queue := ctx.CreateCommandQueue(nil)
	ev, err := queue.EnqueueNDRangeKernel(kernel, []int{256}, nil, nil)
	if err != nil {
		t.Fatalf("EnqueueNDRangeKernel auto local size failed: %v", err)
	}
	if ev == nil {
		t.Fatal("expected non-nil event")
	}
}

func TestCLEnqueueNDRangeKernel2D(t *testing.T) {
	ctx := newCLContext(t)
	prog := ctx.CreateProgramWithSource("src")
	prog.Build(nil, "")
	kernel, _ := prog.CreateKernel("k2d")
	buf, _ := ctx.CreateBuffer(CLMemReadWrite, 64, nil)
	kernel.SetArg(0, buf)

	queue := ctx.CreateCommandQueue(nil)
	_, err := queue.EnqueueNDRangeKernel(kernel, []int{64, 64}, []int{8, 8}, nil)
	if err != nil {
		t.Fatalf("2D EnqueueNDRangeKernel failed: %v", err)
	}
}

func TestCLEnqueueNDRangeKernel3D(t *testing.T) {
	ctx := newCLContext(t)
	prog := ctx.CreateProgramWithSource("src")
	prog.Build(nil, "")
	kernel, _ := prog.CreateKernel("k3d")
	buf, _ := ctx.CreateBuffer(CLMemReadWrite, 64, nil)
	kernel.SetArg(0, buf)

	queue := ctx.CreateCommandQueue(nil)
	_, err := queue.EnqueueNDRangeKernel(kernel, []int{32, 32, 32}, []int{4, 4, 4}, nil)
	if err != nil {
		t.Fatalf("3D EnqueueNDRangeKernel failed: %v", err)
	}
}

func TestCLFinishAndFlush(t *testing.T) {
	ctx := newCLContext(t)
	queue := ctx.CreateCommandQueue(nil)
	queue.Finish() // Should not panic
	queue.Flush()  // Should not panic
}

func TestCLEventWait(t *testing.T) {
	ctx := newCLContext(t)
	buf, _ := ctx.CreateBuffer(CLMemReadWrite, 4, nil)
	queue := ctx.CreateCommandQueue(nil)
	ev, _ := queue.EnqueueWriteBuffer(buf, 0, 4, []byte{1, 2, 3, 4}, nil)
	ev.Wait() // Should not panic
}

// =========================================================================
// CLPlatform tests
// =========================================================================

func TestCLPlatformGetPlatforms(t *testing.T) {
	platforms, err := GetPlatforms()
	if err != nil {
		t.Fatalf("GetPlatforms failed: %v", err)
	}
	if len(platforms) != 1 {
		t.Errorf("expected 1 platform, got %d", len(platforms))
	}
}

func TestCLPlatformProperties(t *testing.T) {
	platform, _ := NewCLPlatform()
	if platform.Name() == "" {
		t.Error("expected non-empty platform name")
	}
	if platform.Vendor() == "" {
		t.Error("expected non-empty platform vendor")
	}
	if platform.Version() == "" {
		t.Error("expected non-empty platform version")
	}
}

func TestCLPlatformGetDevicesAll(t *testing.T) {
	platform, _ := NewCLPlatform()
	devices := platform.GetDevices(CLDeviceTypeAll)
	if len(devices) == 0 {
		t.Error("expected at least one device")
	}
}

func TestCLPlatformGetDevicesGPU(t *testing.T) {
	platform, _ := NewCLPlatform()
	devices := platform.GetDevices(CLDeviceTypeGPU)
	for _, d := range devices {
		if d.DeviceType() != CLDeviceTypeGPU {
			t.Errorf("expected GPU device, got %d", d.DeviceType())
		}
	}
}

func TestCLPlatformGetDevicesAccelerator(t *testing.T) {
	platform, _ := NewCLPlatform()
	devices := platform.GetDevices(CLDeviceTypeAccelerator)
	// May or may not have accelerator devices
	for _, d := range devices {
		if d.DeviceType() != CLDeviceTypeAccelerator {
			t.Errorf("expected accelerator device, got %d", d.DeviceType())
		}
	}
}
