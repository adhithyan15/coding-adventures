package vendorapisimulators

// Vulkan Runtime Simulator -- the thinnest wrapper over Layer 5.
//
// # What is Vulkan?
//
// Vulkan is the Khronos Group's low-level, cross-platform GPU API. It is the
// most explicit GPU API -- you manage everything: memory types, command buffer
// recording, queue submission, synchronization barriers, descriptor set layouts.
//
// Because our Layer 5 compute runtime is already Vulkan-inspired, this
// simulator is the thinnest wrapper of all six. It mainly adds:
//
//  1. Vulkan naming conventions (the Vk prefix on all types)
//  2. Vulkan-specific structures (VkBufferCreateInfo, VkSubmitInfo, etc.)
//  3. VkResult return codes instead of Go errors
//  4. VkCommandPool for grouping command buffers
//
// # Why Vulkan is So Verbose
//
// Vulkan forces you to be explicit about everything because:
//
//  1. No hidden allocations -- you control every byte of memory
//  2. No implicit sync -- you insert every barrier yourself
//  3. No automatic resource tracking -- you free what you allocate
//  4. No driver guessing -- you tell the driver exactly what you need
//
// # Structure of a Vulkan Program
//
//  1. VkInstance -> VkPhysicalDevice -> VkDevice -> VkQueue
//  2. VkBuffer + VkDeviceMemory (allocate + bind)
//  3. VkShaderModule -> VkPipeline + VkDescriptorSet
//  4. VkCommandPool -> VkCommandBuffer (record commands)
//  5. VkQueueSubmit() + VkFence (execute + synchronize)

import (
	"fmt"

	cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// Suppress unused import for gpucore.
var _ = []gpucore.Instruction(nil)

// =========================================================================
// Vulkan enums
// =========================================================================

// VkResult is a Vulkan function return code.
type VkResult int

const (
	VkSuccess                    VkResult = 0
	VkNotReady                   VkResult = 1
	VkTimeout                    VkResult = 2
	VkErrorOutOfDeviceMemory     VkResult = -3
	VkErrorDeviceLost            VkResult = -4
	VkErrorInitializationFailed  VkResult = -5
)

// VkPipelineBindPoint identifies which pipeline type to bind.
type VkPipelineBindPoint int

const (
	VkPipelineBindPointCompute VkPipelineBindPoint = iota
)

// VkBufferUsageFlagBits describes how a buffer will be used.
type VkBufferUsageFlagBits int

const (
	VkBufferUsageStorageBuffer VkBufferUsageFlagBits = 1 << iota
	VkBufferUsageUniformBuffer
	VkBufferUsageTransferSrc
	VkBufferUsageTransferDst
)

// VkMemoryPropertyFlagBits describes memory access properties.
type VkMemoryPropertyFlagBits int

const (
	VkMemoryPropertyDeviceLocal  VkMemoryPropertyFlagBits = 1 << iota
	VkMemoryPropertyHostVisible
	VkMemoryPropertyHostCoherent
	VkMemoryPropertyHostCached
)

// VkSharingMode specifies resource sharing mode.
type VkSharingMode int

const (
	VkSharingModeExclusive VkSharingMode = iota
	VkSharingModeConcurrent
)

// =========================================================================
// Vulkan create-info structures
// =========================================================================

// VkBufferCreateInfo holds parameters for creating a VkBuffer.
type VkBufferCreateInfo struct {
	Size        int
	Usage       VkBufferUsageFlagBits
	SharingMode VkSharingMode
}

// VkMemoryAllocateInfo holds parameters for allocating device memory.
type VkMemoryAllocateInfo struct {
	Size            int
	MemoryTypeIndex int
}

// VkShaderModuleCreateInfo holds parameters for creating a shader module.
type VkShaderModuleCreateInfo struct {
	Code []gpucore.Instruction
}

// VkComputePipelineCreateInfo holds parameters for creating a compute pipeline.
type VkComputePipelineCreateInfo struct {
	ShaderStage *VkPipelineShaderStageCreateInfo
	Layout      *VkPipelineLayout
}

// VkPipelineShaderStageCreateInfo holds shader stage configuration.
type VkPipelineShaderStageCreateInfo struct {
	Stage      string
	Module     *VkShaderModule
	EntryPoint string
}

// VkSubmitInfo holds parameters for a queue submission.
type VkSubmitInfo struct {
	CommandBuffers   []*VkCommandBuffer
	WaitSemaphores   []*VkSemaphore
	SignalSemaphores []*VkSemaphore
}

// VkBufferCopy describes a region to copy between buffers.
type VkBufferCopy struct {
	SrcOffset int
	DstOffset int
	Size      int
}

// VkWriteDescriptorSet describes a write to a descriptor set.
type VkWriteDescriptorSet struct {
	DstSet         *VkDescriptorSet
	DstBinding     int
	DescriptorType string
	BufferInfo     *VkDescriptorBufferInfo
}

// VkDescriptorBufferInfo is a buffer reference for descriptor set writes.
type VkDescriptorBufferInfo struct {
	Buffer *VkBuffer
	Offset int
	Range  int
}

// VkCommandPoolCreateInfo holds parameters for creating a command pool.
type VkCommandPoolCreateInfo struct {
	QueueFamilyIndex int
}

// VkDescriptorSetLayoutCreateInfo holds parameters for creating a descriptor set layout.
type VkDescriptorSetLayoutCreateInfo struct {
	Bindings []VkDescriptorSetLayoutBinding
}

// VkDescriptorSetLayoutBinding describes one binding slot in a descriptor set layout.
type VkDescriptorSetLayoutBinding struct {
	Binding         int
	DescriptorType  string
	DescriptorCount int
}

// VkPipelineLayoutCreateInfo holds parameters for creating a pipeline layout.
type VkPipelineLayoutCreateInfo struct {
	SetLayouts       []*VkDescriptorSetLayout
	PushConstantSize int
}

// VkDescriptorSetAllocateInfo holds parameters for allocating descriptor sets.
type VkDescriptorSetAllocateInfo struct {
	SetLayouts []*VkDescriptorSetLayout
}

// =========================================================================
// Vulkan wrapper objects -- thin wrappers over Layer 5
// =========================================================================

// VkPhysicalDevice wraps Layer 5 PhysicalDevice.
type VkPhysicalDevice struct {
	physical *cr.PhysicalDevice
}

// VkGetPhysicalDeviceProperties queries device properties.
func (pd *VkPhysicalDevice) VkGetPhysicalDeviceProperties() map[string]interface{} {
	return map[string]interface{}{
		"device_name": pd.physical.Name(),
		"device_type": pd.physical.DeviceType().String(),
		"vendor":      pd.physical.Vendor(),
	}
}

// VkGetPhysicalDeviceMemoryProperties queries memory properties.
func (pd *VkPhysicalDevice) VkGetPhysicalDeviceMemoryProperties() map[string]interface{} {
	mp := pd.physical.MemProperties()
	heaps := make([]map[string]interface{}, len(mp.Heaps))
	for i, h := range mp.Heaps {
		heaps[i] = map[string]interface{}{
			"size":  h.Size,
			"flags": fmt.Sprintf("%v", h.Flags),
		}
	}
	return map[string]interface{}{
		"heap_count": len(mp.Heaps),
		"heaps":      heaps,
		"is_unified": mp.IsUnified,
	}
}

// VkGetPhysicalDeviceQueueFamilyProperties queries queue family properties.
func (pd *VkPhysicalDevice) VkGetPhysicalDeviceQueueFamilyProperties() []map[string]interface{} {
	var result []map[string]interface{}
	for _, qf := range pd.physical.QueueFamilies() {
		result = append(result, map[string]interface{}{
			"queue_type":  qf.QueueType.String(),
			"queue_count": qf.Count,
		})
	}
	return result
}

// VkBuffer wraps Layer 5 Buffer.
type VkBuffer struct {
	buffer *cr.Buffer
}

// Size returns the buffer size in bytes.
func (b *VkBuffer) Size() int { return b.buffer.Size }

// VkDeviceMemory wraps Layer 5 Buffer's memory.
type VkDeviceMemory struct {
	buffer *cr.Buffer
	mm     *cr.MemoryManager
}

// VkShaderModule wraps Layer 5 ShaderModule.
type VkShaderModule struct {
	shader *cr.ShaderModule
}

// VkPipeline wraps Layer 5 Pipeline.
type VkPipeline struct {
	pipeline *cr.Pipeline
}

// VkDescriptorSetLayout wraps Layer 5 DescriptorSetLayout.
type VkDescriptorSetLayout struct {
	layout *cr.DescriptorSetLayout
}

// VkPipelineLayout wraps Layer 5 PipelineLayout.
type VkPipelineLayout struct {
	layout *cr.PipelineLayout
}

// VkDescriptorSet wraps Layer 5 DescriptorSet.
type VkDescriptorSet struct {
	ds *cr.DescriptorSet
}

// VkFence wraps Layer 5 Fence.
type VkFence struct {
	fence *cr.Fence
}

// Signaled returns whether the fence has been signaled.
func (f *VkFence) Signaled() bool { return f.fence.Signaled() }

// VkSemaphore wraps Layer 5 Semaphore.
type VkSemaphore struct {
	semaphore *cr.Semaphore
}

// VkCommandPool groups command buffers.
type VkCommandPool struct {
	device           *VkDevice
	queueFamilyIndex int
	commandBuffers   []*VkCommandBuffer
}

// VkAllocateCommandBuffers allocates command buffers from this pool.
func (p *VkCommandPool) VkAllocateCommandBuffers(count int) []*VkCommandBuffer {
	cbs := make([]*VkCommandBuffer, count)
	for i := 0; i < count; i++ {
		innerCB := p.device.logical.CreateCommandBuffer()
		vkCB := &VkCommandBuffer{cb: innerCB}
		cbs[i] = vkCB
		p.commandBuffers = append(p.commandBuffers, vkCB)
	}
	return cbs
}

// VkResetCommandPool resets all command buffers in this pool.
func (p *VkCommandPool) VkResetCommandPool() {
	for _, vkCB := range p.commandBuffers {
		vkCB.cb.Reset()
	}
}

// VkFreeCommandBuffers frees specific command buffers back to this pool.
func (p *VkCommandPool) VkFreeCommandBuffers(buffers []*VkCommandBuffer) {
	for _, buf := range buffers {
		for i, existing := range p.commandBuffers {
			if existing == buf {
				p.commandBuffers = append(p.commandBuffers[:i], p.commandBuffers[i+1:]...)
				break
			}
		}
	}
}

// VkCommandBuffer wraps Layer 5 CommandBuffer with vk_ prefix.
type VkCommandBuffer struct {
	cb *cr.CommandBuffer
}

// VkBeginCommandBuffer begins recording.
func (vcb *VkCommandBuffer) VkBeginCommandBuffer(flags int) error {
	return vcb.cb.Begin()
}

// VkEndCommandBuffer ends recording.
func (vcb *VkCommandBuffer) VkEndCommandBuffer() error {
	return vcb.cb.End()
}

// VkCmdBindPipeline binds a pipeline.
func (vcb *VkCommandBuffer) VkCmdBindPipeline(bindPoint VkPipelineBindPoint, pipeline *VkPipeline) error {
	return vcb.cb.CmdBindPipeline(pipeline.pipeline)
}

// VkCmdBindDescriptorSets binds descriptor sets.
func (vcb *VkCommandBuffer) VkCmdBindDescriptorSets(
	bindPoint VkPipelineBindPoint,
	layout *VkPipelineLayout,
	descriptorSets []*VkDescriptorSet,
) error {
	for _, ds := range descriptorSets {
		if err := vcb.cb.CmdBindDescriptorSet(ds.ds); err != nil {
			return err
		}
	}
	return nil
}

// VkCmdPushConstants sets push constants.
func (vcb *VkCommandBuffer) VkCmdPushConstants(layout *VkPipelineLayout, offset int, data []byte) error {
	return vcb.cb.CmdPushConstants(offset, data)
}

// VkCmdDispatch dispatches compute work.
func (vcb *VkCommandBuffer) VkCmdDispatch(x, y, z int) error {
	return vcb.cb.CmdDispatch(x, y, z)
}

// VkCmdCopyBuffer copies between buffers.
func (vcb *VkCommandBuffer) VkCmdCopyBuffer(src, dst *VkBuffer, regions []VkBufferCopy) error {
	for _, region := range regions {
		if err := vcb.cb.CmdCopyBuffer(
			src.buffer, dst.buffer, region.Size,
			region.SrcOffset, region.DstOffset,
		); err != nil {
			return err
		}
	}
	return nil
}

// VkCmdFillBuffer fills buffer with a value.
func (vcb *VkCommandBuffer) VkCmdFillBuffer(buffer *VkBuffer, offset, size, data int) error {
	return vcb.cb.CmdFillBuffer(buffer.buffer, data, offset, size)
}

// VkCmdPipelineBarrier inserts a pipeline barrier.
func (vcb *VkCommandBuffer) VkCmdPipelineBarrier(srcStage, dstStage string) error {
	return vcb.cb.CmdPipelineBarrier(cr.PipelineBarrierDesc{
		SrcStage: parsePipelineStage(srcStage),
		DstStage: parsePipelineStage(dstStage),
	})
}

// parsePipelineStage converts a string to a PipelineStage.
func parsePipelineStage(s string) cr.PipelineStage {
	switch s {
	case "compute":
		return cr.PipelineStageCompute
	case "transfer":
		return cr.PipelineStageTransfer
	case "host":
		return cr.PipelineStageHost
	case "top_of_pipe":
		return cr.PipelineStageTopOfPipe
	case "bottom_of_pipe":
		return cr.PipelineStageBottomOfPipe
	default:
		return cr.PipelineStageTopOfPipe
	}
}

// =========================================================================
// VkQueue -- wraps Layer 5 CommandQueue
// =========================================================================

// VkQueue wraps Layer 5 CommandQueue.
type VkQueue struct {
	queue *cr.CommandQueue
}

// VkQueueSubmit submits work to the queue.
func (q *VkQueue) VkQueueSubmit(submits []VkSubmitInfo, fence *VkFence) VkResult {
	for _, submit := range submits {
		cbs := make([]*cr.CommandBuffer, len(submit.CommandBuffers))
		for i, vkCB := range submit.CommandBuffers {
			cbs[i] = vkCB.cb
		}
		waitSems := make([]*cr.Semaphore, len(submit.WaitSemaphores))
		for i, s := range submit.WaitSemaphores {
			waitSems[i] = s.semaphore
		}
		signalSems := make([]*cr.Semaphore, len(submit.SignalSemaphores))
		for i, s := range submit.SignalSemaphores {
			signalSems[i] = s.semaphore
		}

		opts := &cr.SubmitOptions{}
		if len(waitSems) > 0 {
			opts.WaitSemaphores = waitSems
		}
		if len(signalSems) > 0 {
			opts.SignalSemaphores = signalSems
		}
		if fence != nil {
			opts.Fence = fence.fence
		}

		_, err := q.queue.Submit(cbs, opts)
		if err != nil {
			return VkErrorDeviceLost
		}
	}
	return VkSuccess
}

// VkQueueWaitIdle waits for all queue work to complete.
func (q *VkQueue) VkQueueWaitIdle() {
	q.queue.WaitIdle()
}

// =========================================================================
// VkDevice -- wraps LogicalDevice
// =========================================================================

// VkDevice wraps Layer 5 LogicalDevice with Vulkan-style API.
type VkDevice struct {
	logical *cr.LogicalDevice
}

// VkGetDeviceQueue gets a queue from the device.
func (d *VkDevice) VkGetDeviceQueue(familyIndex, queueIndex int) *VkQueue {
	familyName := "compute"
	if familyIndex == 1 {
		familyName = "transfer"
	}
	queues := d.logical.Queues()
	if queueList, ok := queues[familyName]; ok && queueIndex < len(queueList) {
		return &VkQueue{queue: queueList[queueIndex]}
	}
	return &VkQueue{queue: queues["compute"][0]}
}

// VkCreateCommandPool creates a command pool.
func (d *VkDevice) VkCreateCommandPool(createInfo VkCommandPoolCreateInfo) *VkCommandPool {
	return &VkCommandPool{device: d, queueFamilyIndex: createInfo.QueueFamilyIndex}
}

// VkAllocateMemory allocates device memory.
func (d *VkDevice) VkAllocateMemory(allocInfo VkMemoryAllocateInfo) (*VkDeviceMemory, error) {
	memType := DefaultMemType()
	if allocInfo.MemoryTypeIndex == 1 {
		memType = cr.MemoryTypeHostVisible | cr.MemoryTypeHostCoherent
	}
	buf, err := d.logical.MemoryManager().Allocate(allocInfo.Size, memType, DefaultUsage())
	if err != nil {
		return nil, err
	}
	return &VkDeviceMemory{buffer: buf, mm: d.logical.MemoryManager()}, nil
}

// VkCreateBuffer creates a buffer.
func (d *VkDevice) VkCreateBuffer(createInfo VkBufferCreateInfo) (*VkBuffer, error) {
	buf, err := d.logical.MemoryManager().Allocate(createInfo.Size, DefaultMemType(), DefaultUsage())
	if err != nil {
		return nil, err
	}
	return &VkBuffer{buffer: buf}, nil
}

// VkBindBufferMemory binds memory to a buffer (no-op in our simulator).
func (d *VkDevice) VkBindBufferMemory(buffer *VkBuffer, memory *VkDeviceMemory, offset int) {}

// VkMapMemory maps device memory for CPU access.
func (d *VkDevice) VkMapMemory(memory *VkDeviceMemory, offset, size int) ([]byte, error) {
	mapped, err := memory.mm.Map(memory.buffer)
	if err != nil {
		return nil, err
	}
	return mapped.GetData(), nil
}

// VkUnmapMemory unmaps device memory.
func (d *VkDevice) VkUnmapMemory(memory *VkDeviceMemory) error {
	if memory.buffer.Mapped {
		return memory.mm.Unmap(memory.buffer)
	}
	return nil
}

// VkCreateShaderModule creates a shader module.
func (d *VkDevice) VkCreateShaderModule(createInfo VkShaderModuleCreateInfo) *VkShaderModule {
	shader := d.logical.CreateShaderModule(cr.ShaderModuleOptions{Code: createInfo.Code})
	return &VkShaderModule{shader: shader}
}

// VkCreateDescriptorSetLayout creates a descriptor set layout.
func (d *VkDevice) VkCreateDescriptorSetLayout(createInfo VkDescriptorSetLayoutCreateInfo) *VkDescriptorSetLayout {
	bindings := make([]cr.DescriptorBinding, len(createInfo.Bindings))
	for i, b := range createInfo.Bindings {
		count := b.DescriptorCount
		if count == 0 {
			count = 1
		}
		bindings[i] = cr.DescriptorBinding{
			Binding: b.Binding,
			Type:    b.DescriptorType,
			Count:   count,
		}
	}
	layout := d.logical.CreateDescriptorSetLayout(bindings)
	return &VkDescriptorSetLayout{layout: layout}
}

// VkCreatePipelineLayout creates a pipeline layout.
func (d *VkDevice) VkCreatePipelineLayout(createInfo VkPipelineLayoutCreateInfo) *VkPipelineLayout {
	layouts := make([]*cr.DescriptorSetLayout, len(createInfo.SetLayouts))
	for i, sl := range createInfo.SetLayouts {
		layouts[i] = sl.layout
	}
	pl := d.logical.CreatePipelineLayout(layouts, createInfo.PushConstantSize)
	return &VkPipelineLayout{layout: pl}
}

// VkCreateComputePipelines creates compute pipelines.
func (d *VkDevice) VkCreateComputePipelines(createInfos []VkComputePipelineCreateInfo) []*VkPipeline {
	var pipelines []*VkPipeline
	for _, ci := range createInfos {
		if ci.ShaderStage != nil && ci.ShaderStage.Module != nil && ci.Layout != nil {
			p := d.logical.CreateComputePipeline(ci.ShaderStage.Module.shader, ci.Layout.layout)
			pipelines = append(pipelines, &VkPipeline{pipeline: p})
		}
	}
	return pipelines
}

// VkAllocateDescriptorSets allocates descriptor sets.
func (d *VkDevice) VkAllocateDescriptorSets(allocInfo VkDescriptorSetAllocateInfo) []*VkDescriptorSet {
	var sets []*VkDescriptorSet
	for _, sl := range allocInfo.SetLayouts {
		ds := d.logical.CreateDescriptorSet(sl.layout)
		sets = append(sets, &VkDescriptorSet{ds: ds})
	}
	return sets
}

// VkUpdateDescriptorSets writes buffer bindings to descriptor sets.
func (d *VkDevice) VkUpdateDescriptorSets(writes []VkWriteDescriptorSet) {
	for _, write := range writes {
		if write.DstSet != nil && write.BufferInfo != nil && write.BufferInfo.Buffer != nil {
			write.DstSet.ds.Write(write.DstBinding, write.BufferInfo.Buffer.buffer)
		}
	}
}

// VkCreateFence creates a fence.
func (d *VkDevice) VkCreateFence(flags int) *VkFence {
	signaled := flags&1 != 0
	fence := d.logical.CreateFence(signaled)
	return &VkFence{fence: fence}
}

// VkCreateSemaphore creates a semaphore.
func (d *VkDevice) VkCreateSemaphore() *VkSemaphore {
	sem := d.logical.CreateSemaphore()
	return &VkSemaphore{semaphore: sem}
}

// VkWaitForFences waits for fences.
func (d *VkDevice) VkWaitForFences(fences []*VkFence, waitAll bool, timeout int) VkResult {
	for _, f := range fences {
		if f.fence.Signaled() {
			if !waitAll {
				return VkSuccess
			}
		} else if waitAll {
			return VkNotReady
		}
	}
	return VkSuccess
}

// VkResetFences resets fences to unsignaled state.
func (d *VkDevice) VkResetFences(fences []*VkFence) {
	for _, f := range fences {
		f.fence.Reset()
	}
}

// VkDeviceWaitIdle waits for all work to complete.
func (d *VkDevice) VkDeviceWaitIdle() {
	d.logical.WaitIdle()
}

// =========================================================================
// VkInstance -- the Vulkan entry point
// =========================================================================

// VkInstance is the Vulkan entry point for device discovery.
type VkInstance struct {
	*BaseVendorSimulator
}

// NewVkInstance creates a Vulkan instance.
func NewVkInstance() (*VkInstance, error) {
	base, err := InitBase(nil, "")
	if err != nil {
		return nil, fmt.Errorf("failed to create Vulkan instance: %w", err)
	}
	return &VkInstance{BaseVendorSimulator: base}, nil
}

// VkEnumeratePhysicalDevices enumerates all physical devices.
func (vi *VkInstance) VkEnumeratePhysicalDevices() []*VkPhysicalDevice {
	var result []*VkPhysicalDevice
	for _, pd := range vi.PhysicalDevices {
		result = append(result, &VkPhysicalDevice{physical: pd})
	}
	return result
}

// VkCreateDevice creates a logical device.
func (vi *VkInstance) VkCreateDevice(physicalDevice *VkPhysicalDevice) *VkDevice {
	logical := vi.Instance.CreateLogicalDevice(physicalDevice.physical, nil)
	return &VkDevice{logical: logical}
}
