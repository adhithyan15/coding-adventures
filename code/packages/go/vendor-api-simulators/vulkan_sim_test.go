package vendorapisimulators

import (
	"testing"

	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Vulkan tests
// =========================================================================

func newVulkan(t *testing.T) *VkInstance {
	t.Helper()
	vi, err := NewVkInstance()
	if err != nil {
		t.Fatalf("NewVkInstance failed: %v", err)
	}
	return vi
}

func TestVkInstanceCreation(t *testing.T) {
	vi := newVulkan(t)
	if vi.Instance == nil {
		t.Fatal("expected non-nil Instance")
	}
}

func TestVkEnumeratePhysicalDevices(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	if len(pds) == 0 {
		t.Fatal("expected at least one physical device")
	}
}

func TestVkPhysicalDeviceProperties(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	props := pds[0].VkGetPhysicalDeviceProperties()
	if props["device_name"] == "" {
		t.Error("expected non-empty device name")
	}
	if props["vendor"] == "" {
		t.Error("expected non-empty vendor")
	}
	if props["device_type"] == "" {
		t.Error("expected non-empty device type")
	}
}

func TestVkPhysicalDeviceMemoryProperties(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	memProps := pds[0].VkGetPhysicalDeviceMemoryProperties()
	if memProps["heap_count"] == nil {
		t.Error("expected heap_count")
	}
}

func TestVkPhysicalDeviceQueueFamilyProperties(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	qfProps := pds[0].VkGetPhysicalDeviceQueueFamilyProperties()
	if len(qfProps) == 0 {
		t.Fatal("expected at least one queue family")
	}
	for _, qf := range qfProps {
		if qf["queue_type"] == "" {
			t.Error("expected non-empty queue type")
		}
	}
}

func TestVkCreateDevice(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	if device == nil {
		t.Fatal("expected non-nil device")
	}
}

func TestVkGetDeviceQueue(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	queue := device.VkGetDeviceQueue(0, 0)
	if queue == nil {
		t.Fatal("expected non-nil queue")
	}
}

func TestVkGetDeviceQueueTransfer(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	// family index 1 maps to transfer
	queue := device.VkGetDeviceQueue(1, 0)
	if queue == nil {
		t.Fatal("expected non-nil transfer queue")
	}
}

func TestVkCreateCommandPool(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{QueueFamilyIndex: 0})
	if pool == nil {
		t.Fatal("expected non-nil command pool")
	}
}

func TestVkAllocateCommandBuffers(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(3)
	if len(cbs) != 3 {
		t.Errorf("expected 3 command buffers, got %d", len(cbs))
	}
}

func TestVkResetCommandPool(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(2)
	cbs[0].VkBeginCommandBuffer(0)
	pool.VkResetCommandPool() // Should not panic
}

func TestVkFreeCommandBuffers(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(3)
	pool.VkFreeCommandBuffers(cbs[:1])
	if len(pool.commandBuffers) != 2 {
		t.Errorf("expected 2 command buffers after free, got %d", len(pool.commandBuffers))
	}
}

func TestVkCommandBufferRecording(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]

	err := cb.VkBeginCommandBuffer(0)
	if err != nil {
		t.Fatalf("VkBeginCommandBuffer failed: %v", err)
	}
	err = cb.VkEndCommandBuffer()
	if err != nil {
		t.Fatalf("VkEndCommandBuffer failed: %v", err)
	}
}

func TestVkCreateBuffer(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	buf, err := device.VkCreateBuffer(VkBufferCreateInfo{
		Size:        256,
		Usage:       VkBufferUsageStorageBuffer,
		SharingMode: VkSharingModeExclusive,
	})
	if err != nil {
		t.Fatalf("VkCreateBuffer failed: %v", err)
	}
	if buf.Size() != 256 {
		t.Errorf("expected size 256, got %d", buf.Size())
	}
}

func TestVkAllocateMemory(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	mem, err := device.VkAllocateMemory(VkMemoryAllocateInfo{Size: 128, MemoryTypeIndex: 0})
	if err != nil {
		t.Fatalf("VkAllocateMemory failed: %v", err)
	}
	if mem == nil {
		t.Fatal("expected non-nil memory")
	}
}

func TestVkAllocateMemoryHostVisible(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	mem, err := device.VkAllocateMemory(VkMemoryAllocateInfo{Size: 64, MemoryTypeIndex: 1})
	if err != nil {
		t.Fatalf("VkAllocateMemory (host visible) failed: %v", err)
	}
	if mem == nil {
		t.Fatal("expected non-nil memory")
	}
}

func TestVkBindBufferMemory(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	buf, _ := device.VkCreateBuffer(VkBufferCreateInfo{Size: 64})
	mem, _ := device.VkAllocateMemory(VkMemoryAllocateInfo{Size: 64})
	// Should not panic (no-op)
	device.VkBindBufferMemory(buf, mem, 0)
}

func TestVkMapUnmapMemory(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	mem, _ := device.VkAllocateMemory(VkMemoryAllocateInfo{Size: 32, MemoryTypeIndex: 0})

	data, err := device.VkMapMemory(mem, 0, 32)
	if err != nil {
		t.Fatalf("VkMapMemory failed: %v", err)
	}
	if data == nil {
		t.Fatal("expected non-nil mapped data")
	}
	err = device.VkUnmapMemory(mem)
	if err != nil {
		t.Fatalf("VkUnmapMemory failed: %v", err)
	}
}

func TestVkCreateShaderModule(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	sm := device.VkCreateShaderModule(VkShaderModuleCreateInfo{
		Code: []gpucore.Instruction{},
	})
	if sm == nil {
		t.Fatal("expected non-nil shader module")
	}
}

func TestVkCreateDescriptorSetLayout(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	layout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{
		Bindings: []VkDescriptorSetLayoutBinding{
			{Binding: 0, DescriptorType: "storage", DescriptorCount: 1},
		},
	})
	if layout == nil {
		t.Fatal("expected non-nil descriptor set layout")
	}
}

func TestVkCreateDescriptorSetLayoutZeroCount(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	// DescriptorCount 0 should default to 1
	layout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{
		Bindings: []VkDescriptorSetLayoutBinding{
			{Binding: 0, DescriptorType: "storage", DescriptorCount: 0},
		},
	})
	if layout == nil {
		t.Fatal("expected non-nil descriptor set layout")
	}
}

func TestVkCreatePipelineLayout(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	dsLayout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{})
	plLayout := device.VkCreatePipelineLayout(VkPipelineLayoutCreateInfo{
		SetLayouts:       []*VkDescriptorSetLayout{dsLayout},
		PushConstantSize: 64,
	})
	if plLayout == nil {
		t.Fatal("expected non-nil pipeline layout")
	}
}

func TestVkCreateComputePipelines(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	sm := device.VkCreateShaderModule(VkShaderModuleCreateInfo{})
	dsLayout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{})
	plLayout := device.VkCreatePipelineLayout(VkPipelineLayoutCreateInfo{
		SetLayouts: []*VkDescriptorSetLayout{dsLayout},
	})
	pipelines := device.VkCreateComputePipelines([]VkComputePipelineCreateInfo{
		{
			ShaderStage: &VkPipelineShaderStageCreateInfo{
				Stage:      "compute",
				Module:     sm,
				EntryPoint: "main",
			},
			Layout: plLayout,
		},
	})
	if len(pipelines) != 1 {
		t.Errorf("expected 1 pipeline, got %d", len(pipelines))
	}
}

func TestVkAllocateDescriptorSets(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	dsLayout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{
		Bindings: []VkDescriptorSetLayoutBinding{
			{Binding: 0, DescriptorType: "storage", DescriptorCount: 1},
		},
	})
	sets := device.VkAllocateDescriptorSets(VkDescriptorSetAllocateInfo{
		SetLayouts: []*VkDescriptorSetLayout{dsLayout},
	})
	if len(sets) != 1 {
		t.Errorf("expected 1 descriptor set, got %d", len(sets))
	}
}

func TestVkUpdateDescriptorSets(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	dsLayout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{
		Bindings: []VkDescriptorSetLayoutBinding{
			{Binding: 0, DescriptorType: "storage", DescriptorCount: 1},
		},
	})
	sets := device.VkAllocateDescriptorSets(VkDescriptorSetAllocateInfo{
		SetLayouts: []*VkDescriptorSetLayout{dsLayout},
	})
	buf, _ := device.VkCreateBuffer(VkBufferCreateInfo{Size: 64})
	device.VkUpdateDescriptorSets([]VkWriteDescriptorSet{
		{
			DstSet:         sets[0],
			DstBinding:     0,
			DescriptorType: "storage",
			BufferInfo:     &VkDescriptorBufferInfo{Buffer: buf, Offset: 0, Range: 64},
		},
	})
}

func TestVkCreateFence(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	fence := device.VkCreateFence(0)
	if fence == nil {
		t.Fatal("expected non-nil fence")
	}
	if fence.Signaled() {
		t.Error("fence should not be signaled initially (flags=0)")
	}
}

func TestVkCreateFenceSignaled(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	fence := device.VkCreateFence(1) // signaled flag
	if !fence.Signaled() {
		t.Error("fence should be signaled initially (flags=1)")
	}
}

func TestVkCreateSemaphore(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	sem := device.VkCreateSemaphore()
	if sem == nil {
		t.Fatal("expected non-nil semaphore")
	}
}

func TestVkWaitForFencesAllSignaled(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	f1 := device.VkCreateFence(1)
	f2 := device.VkCreateFence(1)
	result := device.VkWaitForFences([]*VkFence{f1, f2}, true, 1000)
	if result != VkSuccess {
		t.Errorf("expected VkSuccess, got %d", result)
	}
}

func TestVkWaitForFencesNotReady(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	f1 := device.VkCreateFence(1)
	f2 := device.VkCreateFence(0) // unsignaled
	result := device.VkWaitForFences([]*VkFence{f1, f2}, true, 0)
	if result != VkNotReady {
		t.Errorf("expected VkNotReady, got %d", result)
	}
}

func TestVkWaitForFencesAny(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	f1 := device.VkCreateFence(1)
	f2 := device.VkCreateFence(0)
	result := device.VkWaitForFences([]*VkFence{f1, f2}, false, 0)
	if result != VkSuccess {
		t.Errorf("expected VkSuccess (any), got %d", result)
	}
}

func TestVkResetFences(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	fence := device.VkCreateFence(1)
	device.VkResetFences([]*VkFence{fence})
	if fence.Signaled() {
		t.Error("fence should be unsignaled after reset")
	}
}

func TestVkDeviceWaitIdle(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	device.VkDeviceWaitIdle() // Should not panic
}

func TestVkQueueSubmit(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	queue := device.VkGetDeviceQueue(0, 0)
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]

	cb.VkBeginCommandBuffer(0)
	cb.VkCmdDispatch(1, 1, 1)
	cb.VkEndCommandBuffer()

	fence := device.VkCreateFence(0)
	result := queue.VkQueueSubmit([]VkSubmitInfo{
		{CommandBuffers: []*VkCommandBuffer{cb}},
	}, fence)
	if result != VkSuccess {
		t.Errorf("expected VkSuccess, got %d", result)
	}
}

func TestVkQueueSubmitWithSignalSemaphore(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	queue := device.VkGetDeviceQueue(0, 0)
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]

	cb.VkBeginCommandBuffer(0)
	cb.VkEndCommandBuffer()

	signal := device.VkCreateSemaphore()
	result := queue.VkQueueSubmit([]VkSubmitInfo{
		{
			CommandBuffers:   []*VkCommandBuffer{cb},
			SignalSemaphores: []*VkSemaphore{signal},
		},
	}, nil)
	if result != VkSuccess {
		t.Errorf("expected VkSuccess, got %d", result)
	}
}

func TestVkQueueWaitIdle(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	queue := device.VkGetDeviceQueue(0, 0)
	queue.VkQueueWaitIdle() // Should not panic
}

func TestVkCmdBindPipeline(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	sm := device.VkCreateShaderModule(VkShaderModuleCreateInfo{})
	dsLayout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{})
	plLayout := device.VkCreatePipelineLayout(VkPipelineLayoutCreateInfo{
		SetLayouts: []*VkDescriptorSetLayout{dsLayout},
	})
	pipelines := device.VkCreateComputePipelines([]VkComputePipelineCreateInfo{
		{
			ShaderStage: &VkPipelineShaderStageCreateInfo{Module: sm},
			Layout:      plLayout,
		},
	})
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]
	cb.VkBeginCommandBuffer(0)
	err := cb.VkCmdBindPipeline(VkPipelineBindPointCompute, pipelines[0])
	if err != nil {
		t.Fatalf("VkCmdBindPipeline failed: %v", err)
	}
}

func TestVkCmdBindDescriptorSets(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	dsLayout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{})
	plLayout := device.VkCreatePipelineLayout(VkPipelineLayoutCreateInfo{
		SetLayouts: []*VkDescriptorSetLayout{dsLayout},
	})
	sets := device.VkAllocateDescriptorSets(VkDescriptorSetAllocateInfo{
		SetLayouts: []*VkDescriptorSetLayout{dsLayout},
	})
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]
	cb.VkBeginCommandBuffer(0)
	err := cb.VkCmdBindDescriptorSets(VkPipelineBindPointCompute, plLayout, sets)
	if err != nil {
		t.Fatalf("VkCmdBindDescriptorSets failed: %v", err)
	}
}

func TestVkCmdPushConstants(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	dsLayout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{})
	plLayout := device.VkCreatePipelineLayout(VkPipelineLayoutCreateInfo{
		SetLayouts: []*VkDescriptorSetLayout{dsLayout},
	})
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]
	cb.VkBeginCommandBuffer(0)
	err := cb.VkCmdPushConstants(plLayout, 0, []byte{1, 2, 3, 4})
	if err != nil {
		t.Fatalf("VkCmdPushConstants failed: %v", err)
	}
}

func TestVkCmdCopyBuffer(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	src, _ := device.VkCreateBuffer(VkBufferCreateInfo{Size: 64})
	dst, _ := device.VkCreateBuffer(VkBufferCreateInfo{Size: 64})
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]
	cb.VkBeginCommandBuffer(0)
	err := cb.VkCmdCopyBuffer(src, dst, []VkBufferCopy{
		{SrcOffset: 0, DstOffset: 0, Size: 64},
	})
	if err != nil {
		t.Fatalf("VkCmdCopyBuffer failed: %v", err)
	}
}

func TestVkCmdFillBuffer(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	buf, _ := device.VkCreateBuffer(VkBufferCreateInfo{Size: 32})
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]
	cb.VkBeginCommandBuffer(0)
	err := cb.VkCmdFillBuffer(buf, 0, 32, 0xFF)
	if err != nil {
		t.Fatalf("VkCmdFillBuffer failed: %v", err)
	}
}

func TestVkCmdPipelineBarrier(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]
	cb.VkBeginCommandBuffer(0)
	err := cb.VkCmdPipelineBarrier("compute", "transfer")
	if err != nil {
		t.Fatalf("VkCmdPipelineBarrier failed: %v", err)
	}
}

func TestParsePipelineStage(t *testing.T) {
	stages := []string{"compute", "transfer", "host", "top_of_pipe", "bottom_of_pipe", "unknown"}
	for _, s := range stages {
		// parsePipelineStage should not panic for any input
		_ = parsePipelineStage(s)
	}
}

func TestVkFullWorkflow(t *testing.T) {
	vi := newVulkan(t)
	pds := vi.VkEnumeratePhysicalDevices()
	device := vi.VkCreateDevice(pds[0])
	queue := device.VkGetDeviceQueue(0, 0)

	// Create buffer
	buf, _ := device.VkCreateBuffer(VkBufferCreateInfo{Size: 64, Usage: VkBufferUsageStorageBuffer})

	// Create shader and pipeline
	sm := device.VkCreateShaderModule(VkShaderModuleCreateInfo{})
	dsLayout := device.VkCreateDescriptorSetLayout(VkDescriptorSetLayoutCreateInfo{
		Bindings: []VkDescriptorSetLayoutBinding{
			{Binding: 0, DescriptorType: "storage", DescriptorCount: 1},
		},
	})
	plLayout := device.VkCreatePipelineLayout(VkPipelineLayoutCreateInfo{
		SetLayouts: []*VkDescriptorSetLayout{dsLayout},
	})
	pipelines := device.VkCreateComputePipelines([]VkComputePipelineCreateInfo{
		{
			ShaderStage: &VkPipelineShaderStageCreateInfo{Module: sm, EntryPoint: "main"},
			Layout:      plLayout,
		},
	})

	// Create descriptor sets
	sets := device.VkAllocateDescriptorSets(VkDescriptorSetAllocateInfo{
		SetLayouts: []*VkDescriptorSetLayout{dsLayout},
	})
	device.VkUpdateDescriptorSets([]VkWriteDescriptorSet{
		{
			DstSet:         sets[0],
			DstBinding:     0,
			DescriptorType: "storage",
			BufferInfo:     &VkDescriptorBufferInfo{Buffer: buf, Offset: 0, Range: 64},
		},
	})

	// Record command buffer
	pool := device.VkCreateCommandPool(VkCommandPoolCreateInfo{})
	cbs := pool.VkAllocateCommandBuffers(1)
	cb := cbs[0]
	cb.VkBeginCommandBuffer(0)
	cb.VkCmdBindPipeline(VkPipelineBindPointCompute, pipelines[0])
	cb.VkCmdBindDescriptorSets(VkPipelineBindPointCompute, plLayout, sets)
	cb.VkCmdDispatch(4, 1, 1)
	cb.VkEndCommandBuffer()

	// Submit
	fence := device.VkCreateFence(0)
	result := queue.VkQueueSubmit([]VkSubmitInfo{
		{CommandBuffers: []*VkCommandBuffer{cb}},
	}, fence)
	if result != VkSuccess {
		t.Fatalf("VkQueueSubmit failed: %d", result)
	}
	queue.VkQueueWaitIdle()
}

func TestVkResultConstants(t *testing.T) {
	if VkSuccess != 0 {
		t.Error("VkSuccess should be 0")
	}
	if VkErrorDeviceLost >= 0 {
		t.Error("VkErrorDeviceLost should be negative")
	}
}

func TestVkBufferUsageFlags(t *testing.T) {
	combined := VkBufferUsageStorageBuffer | VkBufferUsageTransferSrc | VkBufferUsageTransferDst
	if combined&VkBufferUsageStorageBuffer == 0 {
		t.Error("expected storage buffer flag")
	}
	if combined&VkBufferUsageTransferSrc == 0 {
		t.Error("expected transfer src flag")
	}
}

func TestVkSharingModes(t *testing.T) {
	if VkSharingModeExclusive != 0 {
		t.Error("exclusive should be 0")
	}
	if VkSharingModeConcurrent != 1 {
		t.Error("concurrent should be 1")
	}
}
