package computeruntime

import (
	"testing"

	devicesimulator "github.com/adhithyan15/coding-adventures/code/packages/go/device-simulator"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// RuntimeInstance tests
// =========================================================================

func TestRuntimeInstanceDefault(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	if inst.Version() != "0.1.0" {
		t.Errorf("Version = %q, want %q", inst.Version(), "0.1.0")
	}
	devices := inst.EnumeratePhysicalDevices()
	if len(devices) != 5 {
		t.Errorf("expected 5 default devices, got %d", len(devices))
	}
}

func TestRuntimeInstanceCustomDevices(t *testing.T) {
	gpu := devicesimulator.NewNvidiaGPU(nil, 2)
	entries := []DeviceEntry{
		{Accelerator: gpu, Type: DeviceTypeGPU, Vendor: "nvidia"},
	}
	inst := NewRuntimeInstance(entries)
	devices := inst.EnumeratePhysicalDevices()
	if len(devices) != 1 {
		t.Errorf("expected 1 device, got %d", len(devices))
	}
	if devices[0].Vendor() != "nvidia" {
		t.Errorf("Vendor = %q, want %q", devices[0].Vendor(), "nvidia")
	}
}

// =========================================================================
// PhysicalDevice tests
// =========================================================================

func TestPhysicalDeviceProperties(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()

	nvidia := devices[0]
	if nvidia.Vendor() != "nvidia" {
		t.Errorf("Vendor = %q, want %q", nvidia.Vendor(), "nvidia")
	}
	if nvidia.DeviceType() != DeviceTypeGPU {
		t.Errorf("DeviceType = %v, want %v", nvidia.DeviceType(), DeviceTypeGPU)
	}
	if nvidia.Name() == "" {
		t.Error("Name should not be empty")
	}
	if nvidia.DeviceID() != 0 {
		t.Errorf("DeviceID = %d, want 0", nvidia.DeviceID())
	}
}

func TestPhysicalDeviceMemoryProperties(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()

	// NVIDIA (discrete) should have 2 heaps
	nvidia := devices[0]
	props := nvidia.MemProperties()
	if props.IsUnified {
		t.Error("NVIDIA should not be unified memory")
	}
	if len(props.Heaps) != 2 {
		t.Errorf("NVIDIA should have 2 heaps, got %d", len(props.Heaps))
	}

	// Apple ANE (unified) should have 1 heap
	apple := devices[4]
	appleProps := apple.MemProperties()
	if !appleProps.IsUnified {
		t.Error("Apple ANE should be unified memory")
	}
	if len(appleProps.Heaps) != 1 {
		t.Errorf("Apple should have 1 heap, got %d", len(appleProps.Heaps))
	}
}

func TestPhysicalDeviceQueueFamilies(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()

	// NVIDIA (discrete) should have compute + transfer families
	nvidia := devices[0]
	families := nvidia.QueueFamilies()
	if len(families) != 2 {
		t.Errorf("NVIDIA should have 2 queue families, got %d", len(families))
	}

	// Apple (unified) should have only compute family
	apple := devices[4]
	appleFamilies := apple.QueueFamilies()
	if len(appleFamilies) != 1 {
		t.Errorf("Apple should have 1 queue family, got %d", len(appleFamilies))
	}
}

func TestPhysicalDeviceSupportsFeature(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()

	nvidia := devices[0]
	if !nvidia.SupportsFeature("fp32") {
		t.Error("should support fp32")
	}
	if !nvidia.SupportsFeature("fp16") {
		t.Error("should support fp16")
	}
	if nvidia.SupportsFeature("unified_memory") {
		t.Error("NVIDIA should not support unified_memory")
	}
	if !nvidia.SupportsFeature("transfer_queue") {
		t.Error("NVIDIA should support transfer_queue")
	}
	if nvidia.SupportsFeature("nonexistent") {
		t.Error("should not support nonexistent feature")
	}

	apple := devices[4]
	if !apple.SupportsFeature("unified_memory") {
		t.Error("Apple should support unified_memory")
	}
	if apple.SupportsFeature("transfer_queue") {
		t.Error("Apple should not have separate transfer_queue")
	}
}

func TestPhysicalDeviceLimits(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	limits := devices[0].Limits()
	if limits.MaxWorkgroupSize[0] != 1024 {
		t.Errorf("MaxWorkgroupSize[0] = %d, want 1024", limits.MaxWorkgroupSize[0])
	}
}

func TestPhysicalDeviceAccelerator(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	if devices[0].Accelerator() == nil {
		t.Error("Accelerator() should not be nil")
	}
}

// =========================================================================
// LogicalDevice tests
// =========================================================================

func TestCreateLogicalDeviceDefault(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()

	device := inst.CreateLogicalDevice(devices[0], nil)
	if device.PhysicalDevice() != devices[0] {
		t.Error("PhysicalDevice should match")
	}
	if device.MemoryManager() == nil {
		t.Error("MemoryManager should not be nil")
	}
	if device.Stats() == nil {
		t.Error("Stats should not be nil")
	}
	queues := device.Queues()
	if _, ok := queues["compute"]; !ok {
		t.Error("should have compute queues")
	}
	if len(queues["compute"]) != 1 {
		t.Errorf("should have 1 compute queue, got %d", len(queues["compute"]))
	}
}

func TestCreateLogicalDeviceCustomQueues(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()

	device := inst.CreateLogicalDevice(devices[0], []QueueRequest{
		{Type: "compute", Count: 2},
		{Type: "transfer", Count: 1},
	})

	queues := device.Queues()
	if len(queues["compute"]) != 2 {
		t.Errorf("compute queues = %d, want 2", len(queues["compute"]))
	}
	if len(queues["transfer"]) != 1 {
		t.Errorf("transfer queues = %d, want 1", len(queues["transfer"]))
	}
}

// =========================================================================
// LogicalDevice factory method tests
// =========================================================================

func TestLogicalDeviceCreateCommandBuffer(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	device := inst.CreateLogicalDevice(devices[0], nil)

	cb := device.CreateCommandBuffer()
	if cb == nil {
		t.Error("CreateCommandBuffer should not return nil")
	}
	if cb.State() != CommandBufferStateInitial {
		t.Error("new CB should be in INITIAL state")
	}
}

func TestLogicalDeviceCreateShaderModule(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	device := inst.CreateLogicalDevice(devices[0], nil)

	shader := device.CreateShaderModule(ShaderModuleOptions{Operation: "test"})
	if shader == nil {
		t.Error("CreateShaderModule should not return nil")
	}
}

func TestLogicalDeviceCreatePipeline(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	device := inst.CreateLogicalDevice(devices[0], nil)

	shader := device.CreateShaderModule(ShaderModuleOptions{Operation: "test"})
	dsLayout := device.CreateDescriptorSetLayout(nil)
	plLayout := device.CreatePipelineLayout([]*DescriptorSetLayout{dsLayout}, 0)
	pipeline := device.CreateComputePipeline(shader, plLayout)

	if pipeline == nil {
		t.Error("CreateComputePipeline should not return nil")
	}
}

func TestLogicalDeviceCreateDescriptorSet(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	device := inst.CreateLogicalDevice(devices[0], nil)

	dsLayout := device.CreateDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := device.CreateDescriptorSet(dsLayout)
	if ds == nil {
		t.Error("CreateDescriptorSet should not return nil")
	}
}

func TestLogicalDeviceCreateFence(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	device := inst.CreateLogicalDevice(devices[0], nil)

	fence := device.CreateFence(false)
	if fence.Signaled() {
		t.Error("new fence should not be signaled")
	}
}

func TestLogicalDeviceCreateSemaphore(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	device := inst.CreateLogicalDevice(devices[0], nil)

	sem := device.CreateSemaphore()
	if sem == nil {
		t.Error("CreateSemaphore should not return nil")
	}
}

func TestLogicalDeviceCreateEvent(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	device := inst.CreateLogicalDevice(devices[0], nil)

	event := device.CreateEvent()
	if event == nil {
		t.Error("CreateEvent should not return nil")
	}
}

func TestLogicalDeviceWaitIdle(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	device := inst.CreateLogicalDevice(devices[0], nil)
	// Should not panic
	device.WaitIdle()
}

func TestLogicalDeviceReset(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	device := inst.CreateLogicalDevice(devices[0], nil)
	// Should not panic
	device.ResetDevice()
}

// =========================================================================
// End-to-end test
// =========================================================================

func TestEndToEndDispatch(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()
	nvidia := devices[0]

	device := inst.CreateLogicalDevice(nvidia, []QueueRequest{
		{Type: "compute", Count: 1},
	})

	queue := device.Queues()["compute"][0]
	mm := device.MemoryManager()

	// Allocate a buffer
	memType := MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent
	buf, err := mm.Allocate(256, memType, BufferUsageStorage)
	if err != nil {
		t.Fatalf("Allocate failed: %v", err)
	}

	// Create pipeline
	shader := device.CreateShaderModule(ShaderModuleOptions{
		Code:      []gpucore.Instruction{gpucore.Halt()},
		LocalSize: [3]int{1, 1, 1},
	})
	dsLayout := device.CreateDescriptorSetLayout(nil)
	plLayout := device.CreatePipelineLayout([]*DescriptorSetLayout{dsLayout}, 0)
	pipeline := device.CreateComputePipeline(shader, plLayout)

	// Record command buffer
	cb := device.CreateCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdBindPipeline(pipeline)
	_ = cb.CmdDispatch(1, 1, 1)
	_ = cb.End()

	// Submit with fence
	fence := device.CreateFence(false)
	_, err = queue.Submit([]*CommandBuffer{cb}, &SubmitOptions{Fence: fence})
	if err != nil {
		t.Fatalf("Submit failed: %v", err)
	}

	if !fence.Signaled() {
		t.Error("fence should be signaled after submit")
	}
	if !fence.Wait(nil) {
		t.Error("Wait should return true")
	}

	// Check stats
	stats := device.Stats()
	if stats.TotalSubmissions != 1 {
		t.Errorf("TotalSubmissions = %d, want 1", stats.TotalSubmissions)
	}
	if stats.TotalDispatches != 1 {
		t.Errorf("TotalDispatches = %d, want 1", stats.TotalDispatches)
	}

	// Free the buffer
	_ = mm.Free(buf)
}

func TestEnumerateAllDeviceTypes(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()

	vendorSeen := map[string]bool{}
	for _, d := range devices {
		vendorSeen[d.Vendor()] = true
	}

	expected := []string{"nvidia", "amd", "google", "intel", "apple"}
	for _, v := range expected {
		if !vendorSeen[v] {
			t.Errorf("missing vendor: %s", v)
		}
	}
}

func TestCreateLogicalDeviceForEachVendor(t *testing.T) {
	inst := NewRuntimeInstance(nil)
	devices := inst.EnumeratePhysicalDevices()

	for _, pd := range devices {
		device := inst.CreateLogicalDevice(pd, nil)
		if device == nil {
			t.Errorf("CreateLogicalDevice for %s returned nil", pd.Vendor())
		}
	}
}
