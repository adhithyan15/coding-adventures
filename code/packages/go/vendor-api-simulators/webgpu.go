package vendorapisimulators

// WebGPU Runtime Simulator -- safe, browser-first GPU programming.
//
// # What is WebGPU?
//
// WebGPU is the modern web GPU API, designed to run safely in browsers.
// It sits on top of Vulkan (Linux/Windows/Android), Metal (macOS/iOS),
// or D3D12 (Windows), providing a safe, portable abstraction.
//
// # Key Simplifications Over Vulkan
//
//  1. Single queue -- device.Queue is all you get. No queue families,
//     no multiple queues. The runtime handles parallelism internally.
//  2. Automatic barriers -- no manual pipeline barriers.
//  3. No memory types -- just usage flags. The runtime picks optimal memory.
//  4. Always validated -- every operation is checked.
//  5. Immutable command buffers -- once encoder.Finish() is called, the
//     GPUCommandBuffer cannot be modified or re-recorded.
//
// # The WebGPU Object Hierarchy
//
//	GPU (navigator.gpu in browsers)
//	  |-- GPUAdapter (represents a physical device)
//	      |-- GPUDevice (the usable handle)
//	          |-- device.Queue (GPUQueue -- single queue!)
//	          |-- CreateBuffer() -> GPUBuffer
//	          |-- CreateShaderModule() -> GPUShaderModule
//	          |-- CreateComputePipeline() -> GPUComputePipeline
//	          |-- CreateBindGroup() -> GPUBindGroup
//	          |-- CreateCommandEncoder() -> GPUCommandEncoder
//	              |-- BeginComputePass() -> GPUComputePassEncoder
//	              |-- Finish() -> GPUCommandBuffer (frozen!)
//
// # Bind Groups (WebGPU's Descriptor Sets)
//
// WebGPU uses "bind groups" instead of "descriptor sets" -- same concept,
// friendlier name.

import (
	"fmt"

	cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// Suppress unused import for gpucore.
var _ = gpucore.Instruction{}

// =========================================================================
// WebGPU flags
// =========================================================================

// GPUBufferUsage is WebGPU buffer usage flags.
type GPUBufferUsage int

const (
	GPUBufferUsageMapRead  GPUBufferUsage = 1 << iota
	GPUBufferUsageMapWrite
	GPUBufferUsageCopySrc
	GPUBufferUsageCopyDst
	GPUBufferUsageStorage
	GPUBufferUsageUniform
)

// GPUMapMode is WebGPU buffer map modes.
type GPUMapMode int

const (
	GPUMapModeRead GPUMapMode = iota
	GPUMapModeWrite
)

// =========================================================================
// WebGPU descriptor types
// =========================================================================

// GPUBufferDescriptor holds parameters for creating a GPUBuffer.
type GPUBufferDescriptor struct {
	Size             int
	Usage            GPUBufferUsage
	MappedAtCreation bool
}

// GPUShaderModuleDescriptor holds parameters for creating a GPUShaderModule.
type GPUShaderModuleDescriptor struct {
	Code interface{}
}

// GPUProgrammableStage is a shader stage specification for pipeline creation.
type GPUProgrammableStage struct {
	Module     *GPUShaderModule
	EntryPoint string
}

// GPUComputePipelineDescriptor holds parameters for creating a compute pipeline.
type GPUComputePipelineDescriptor struct {
	Layout  interface{} // string "auto" or *GPUPipelineLayout
	Compute *GPUProgrammableStage
}

// GPUBindGroupLayoutEntry is one entry in a bind group layout.
type GPUBindGroupLayoutEntry struct {
	Binding    int
	Visibility int
	BufferType string // "storage", "uniform", "read-only-storage"
}

// GPUBindGroupLayoutDescriptor holds parameters for creating a bind group layout.
type GPUBindGroupLayoutDescriptor struct {
	Entries []GPUBindGroupLayoutEntry
}

// GPUBindGroupEntry is one entry in a bind group (binding index -> buffer).
type GPUBindGroupEntry struct {
	Binding  int
	Resource *GPUBuffer
}

// GPUBindGroupDescriptor holds parameters for creating a bind group.
type GPUBindGroupDescriptor struct {
	Layout  *GPUBindGroupLayout
	Entries []GPUBindGroupEntry
}

// GPUPipelineLayoutDescriptor holds parameters for creating a pipeline layout.
type GPUPipelineLayoutDescriptor struct {
	BindGroupLayouts []*GPUBindGroupLayout
}

// GPURequestAdapterOptions holds options for adapter selection.
type GPURequestAdapterOptions struct {
	PowerPreference string // "low-power" or "high-performance"
}

// GPUAdapterLimits holds hardware limits reported by an adapter.
type GPUAdapterLimits struct {
	MaxBufferSize              int
	MaxComputeWorkgroupSizeX   int
}

// GPUDeviceLimits holds device limits.
type GPUDeviceLimits struct {
	MaxBufferSize              int
	MaxComputeWorkgroupSizeX   int
}

// =========================================================================
// WebGPU wrapper objects
// =========================================================================

// GPUBuffer is a WebGPU buffer -- memory on the device.
//
// WebGPU buffers do not expose memory types. You specify usage flags, and
// the runtime picks the optimal memory type.
type GPUBuffer struct {
	buffer     *cr.Buffer
	mm         *cr.MemoryManager
	size       int
	usage      GPUBufferUsage
	mapped     bool
	mappedData []byte
	destroyed  bool
}

// Size returns buffer size in bytes.
func (b *GPUBuffer) Size() int { return b.size }

// Usage returns buffer usage flags.
func (b *GPUBuffer) Usage() GPUBufferUsage { return b.usage }

// MapAsync maps the buffer for CPU access (simulated as synchronous).
func (b *GPUBuffer) MapAsync(mode GPUMapMode, offset, size int) error {
	if b.destroyed {
		return fmt.Errorf("cannot map a destroyed buffer")
	}
	actualSize := size
	if actualSize <= 0 {
		actualSize = b.size
	}
	if err := b.mm.Invalidate(b.buffer, 0, 0); err != nil {
		return err
	}
	data := b.mm.GetBufferData(b.buffer.BufferID)
	end := offset + actualSize
	if end > len(data) {
		end = len(data)
	}
	b.mappedData = make([]byte, end-offset)
	copy(b.mappedData, data[offset:end])
	b.mapped = true
	return nil
}

// GetMappedRange returns a view of the mapped buffer data.
func (b *GPUBuffer) GetMappedRange(offset, size int) ([]byte, error) {
	if !b.mapped || b.mappedData == nil {
		return nil, fmt.Errorf("buffer is not mapped. Call MapAsync() first")
	}
	actualSize := size
	if actualSize <= 0 {
		actualSize = len(b.mappedData)
	}
	end := offset + actualSize
	if end > len(b.mappedData) {
		end = len(b.mappedData)
	}
	return b.mappedData[offset:end], nil
}

// Unmap unmaps the buffer, making it usable by the GPU again.
func (b *GPUBuffer) Unmap() error {
	if !b.mapped {
		return fmt.Errorf("buffer is not mapped")
	}
	if b.mappedData != nil {
		mapped, err := b.mm.Map(b.buffer)
		if err != nil {
			return err
		}
		if err := mapped.Write(0, b.mappedData); err != nil {
			return err
		}
		if err := b.mm.Unmap(b.buffer); err != nil {
			return err
		}
	}
	b.mapped = false
	b.mappedData = nil
	return nil
}

// Destroy destroys this buffer, releasing its memory.
func (b *GPUBuffer) Destroy() error {
	if !b.destroyed {
		b.destroyed = true
		return b.mm.Free(b.buffer)
	}
	return nil
}

// GPUShaderModule wraps Layer 5 ShaderModule.
type GPUShaderModule struct {
	shader *cr.ShaderModule
}

// GPUBindGroupLayout wraps Layer 5 DescriptorSetLayout.
type GPUBindGroupLayout struct {
	layout *cr.DescriptorSetLayout
}

// GPUPipelineLayout wraps Layer 5 PipelineLayout.
type GPUPipelineLayout struct {
	layout *cr.PipelineLayout
}

// GPUComputePipeline wraps Layer 5 Pipeline.
type GPUComputePipeline struct {
	pipeline         *cr.Pipeline
	bindGroupLayouts []*GPUBindGroupLayout
}

// GetBindGroupLayout returns the bind group layout at a given index.
func (p *GPUComputePipeline) GetBindGroupLayout(index int) (*GPUBindGroupLayout, error) {
	if index < len(p.bindGroupLayouts) {
		return p.bindGroupLayouts[index], nil
	}
	return nil, fmt.Errorf("bind group layout index %d out of range", index)
}

// GPUBindGroup wraps Layer 5 DescriptorSet.
type GPUBindGroup struct {
	ds *cr.DescriptorSet
}

// GPUCommandBuffer is a frozen WebGPU command buffer -- immutable after Finish().
type GPUCommandBuffer struct {
	cb *cr.CommandBuffer
}

// =========================================================================
// GPUComputePassEncoder -- records compute commands
// =========================================================================

// GPUComputePassEncoder records compute commands in a compute pass scope.
type GPUComputePassEncoder struct {
	encoder    *GPUCommandEncoder
	pipeline   *GPUComputePipeline
	bindGroups map[int]*GPUBindGroup
}

// SetPipeline sets the compute pipeline for this pass.
func (e *GPUComputePassEncoder) SetPipeline(pipeline *GPUComputePipeline) {
	e.pipeline = pipeline
}

// SetBindGroup sets a bind group at the given index.
func (e *GPUComputePassEncoder) SetBindGroup(index int, bindGroup *GPUBindGroup) {
	e.bindGroups[index] = bindGroup
}

// DispatchWorkgroups dispatches compute workgroups.
func (e *GPUComputePassEncoder) DispatchWorkgroups(x, y, z int) error {
	if e.pipeline == nil {
		return fmt.Errorf("no pipeline set")
	}
	cb := e.encoder.cb
	if err := cb.CmdBindPipeline(e.pipeline.pipeline); err != nil {
		return err
	}
	// Bind groups in sorted order
	for i := 0; i < len(e.bindGroups); i++ {
		if bg, ok := e.bindGroups[i]; ok {
			if err := cb.CmdBindDescriptorSet(bg.ds); err != nil {
				return err
			}
		}
	}
	return cb.CmdDispatch(x, y, z)
}

// End ends this compute pass.
func (e *GPUComputePassEncoder) End() {}

// =========================================================================
// GPUCommandEncoder -- records commands into a command buffer
// =========================================================================

// GPUCommandEncoder builds a GPUCommandBuffer.
type GPUCommandEncoder struct {
	device *GPUDevice
	cb     *cr.CommandBuffer
}

// BeginComputePass begins a compute pass.
func (e *GPUCommandEncoder) BeginComputePass() *GPUComputePassEncoder {
	return &GPUComputePassEncoder{
		encoder:    e,
		bindGroups: make(map[int]*GPUBindGroup),
	}
}

// CopyBufferToBuffer copies data between buffers.
func (e *GPUCommandEncoder) CopyBufferToBuffer(
	source *GPUBuffer, sourceOffset int,
	destination *GPUBuffer, destinationOffset int,
	size int,
) error {
	return e.cb.CmdCopyBuffer(
		source.buffer, destination.buffer, size,
		sourceOffset, destinationOffset,
	)
}

// Finish ends recording and produces a frozen command buffer.
func (e *GPUCommandEncoder) Finish() (*GPUCommandBuffer, error) {
	if err := e.cb.End(); err != nil {
		return nil, err
	}
	return &GPUCommandBuffer{cb: e.cb}, nil
}

// =========================================================================
// GPUQueue -- the single submission queue
// =========================================================================

// GPUQueue is the single submission queue on a WebGPU device.
type GPUQueue struct {
	device *GPUDevice
}

// Submit submits command buffers for execution.
func (q *GPUQueue) Submit(commandBuffers []*GPUCommandBuffer) error {
	queue := q.device.ComputeQueue
	for _, gpuCB := range commandBuffers {
		fence := q.device.LogicalDevice.CreateFence(false)
		_, err := queue.Submit([]*cr.CommandBuffer{gpuCB.cb}, &cr.SubmitOptions{Fence: fence})
		if err != nil {
			return err
		}
		fence.Wait(nil)
	}
	return nil
}

// WriteBuffer writes data to a buffer (convenience method).
func (q *GPUQueue) WriteBuffer(buffer *GPUBuffer, bufferOffset int, data []byte) error {
	mm := q.device.MemoryManager
	mapped, err := mm.Map(buffer.buffer)
	if err != nil {
		return err
	}
	if err := mapped.Write(bufferOffset, data); err != nil {
		return err
	}
	return mm.Unmap(buffer.buffer)
}

// =========================================================================
// GPUDevice -- the main WebGPU device
// =========================================================================

// GPUDevice is the main entry point for WebGPU GPU programming.
type GPUDevice struct {
	*BaseVendorSimulator
	Queue    *GPUQueue
	Features map[string]bool
	Limits   GPUDeviceLimits
}

// NewGPUDevice creates a WebGPU device.
func NewGPUDevice(vendorHint string) (*GPUDevice, error) {
	base, err := InitBase(nil, vendorHint)
	if err != nil {
		return nil, fmt.Errorf("failed to create WebGPU device: %w", err)
	}
	dev := &GPUDevice{
		BaseVendorSimulator: base,
		Features: map[string]bool{"compute": true},
		Limits: GPUDeviceLimits{
			MaxBufferSize:            2 * 1024 * 1024 * 1024,
			MaxComputeWorkgroupSizeX: 1024,
		},
	}
	dev.Queue = &GPUQueue{device: dev}
	return dev, nil
}

// CreateBuffer creates a buffer.
func (d *GPUDevice) CreateBuffer(descriptor GPUBufferDescriptor) (*GPUBuffer, error) {
	buf, err := d.MemoryManager.Allocate(descriptor.Size, DefaultMemType(), DefaultUsage())
	if err != nil {
		return nil, err
	}
	gpuBuf := &GPUBuffer{
		buffer: buf,
		mm:     d.MemoryManager,
		size:   descriptor.Size,
		usage:  descriptor.Usage,
	}
	if descriptor.MappedAtCreation {
		if err := gpuBuf.MapAsync(GPUMapModeWrite, 0, 0); err != nil {
			return nil, err
		}
	}
	return gpuBuf, nil
}

// CreateShaderModule creates a shader module.
func (d *GPUDevice) CreateShaderModule(descriptor GPUShaderModuleDescriptor) *GPUShaderModule {
	var code []gpucore.Instruction
	if instrs, ok := descriptor.Code.([]gpucore.Instruction); ok {
		code = instrs
	}
	shader := d.LogicalDevice.CreateShaderModule(cr.ShaderModuleOptions{Code: code})
	return &GPUShaderModule{shader: shader}
}

// CreateComputePipeline creates a compute pipeline.
func (d *GPUDevice) CreateComputePipeline(descriptor GPUComputePipelineDescriptor) *GPUComputePipeline {
	var shader *cr.ShaderModule
	if descriptor.Compute != nil && descriptor.Compute.Module != nil {
		shader = descriptor.Compute.Module.shader
	} else {
		shader = d.LogicalDevice.CreateShaderModule(cr.ShaderModuleOptions{})
	}

	dsLayout := d.LogicalDevice.CreateDescriptorSetLayout(nil)
	plLayout := d.LogicalDevice.CreatePipelineLayout([]*cr.DescriptorSetLayout{dsLayout}, 0)
	pipeline := d.LogicalDevice.CreateComputePipeline(shader, plLayout)

	bgLayout := &GPUBindGroupLayout{layout: dsLayout}
	return &GPUComputePipeline{pipeline: pipeline, bindGroupLayouts: []*GPUBindGroupLayout{bgLayout}}
}

// CreateBindGroupLayout creates a bind group layout.
func (d *GPUDevice) CreateBindGroupLayout(descriptor GPUBindGroupLayoutDescriptor) *GPUBindGroupLayout {
	bindings := make([]cr.DescriptorBinding, len(descriptor.Entries))
	for i, e := range descriptor.Entries {
		bufType := e.BufferType
		if bufType == "" {
			bufType = "storage"
		}
		bindings[i] = cr.DescriptorBinding{Binding: e.Binding, Type: bufType, Count: 1}
	}
	layout := d.LogicalDevice.CreateDescriptorSetLayout(bindings)
	return &GPUBindGroupLayout{layout: layout}
}

// CreatePipelineLayout creates a pipeline layout.
func (d *GPUDevice) CreatePipelineLayout(descriptor GPUPipelineLayoutDescriptor) *GPUPipelineLayout {
	layouts := make([]*cr.DescriptorSetLayout, len(descriptor.BindGroupLayouts))
	for i, bg := range descriptor.BindGroupLayouts {
		layouts[i] = bg.layout
	}
	pl := d.LogicalDevice.CreatePipelineLayout(layouts, 0)
	return &GPUPipelineLayout{layout: pl}
}

// CreateBindGroup creates a bind group (WebGPU's descriptor set).
func (d *GPUDevice) CreateBindGroup(descriptor GPUBindGroupDescriptor) *GPUBindGroup {
	var layout *cr.DescriptorSetLayout
	if descriptor.Layout != nil {
		layout = descriptor.Layout.layout
	} else {
		layout = d.LogicalDevice.CreateDescriptorSetLayout(nil)
	}
	ds := d.LogicalDevice.CreateDescriptorSet(layout)
	for _, entry := range descriptor.Entries {
		if entry.Resource != nil {
			ds.Write(entry.Binding, entry.Resource.buffer)
		}
	}
	return &GPUBindGroup{ds: ds}
}

// CreateCommandEncoder creates a command encoder.
func (d *GPUDevice) CreateCommandEncoder() (*GPUCommandEncoder, error) {
	cb := d.LogicalDevice.CreateCommandBuffer()
	if err := cb.Begin(); err != nil {
		return nil, err
	}
	return &GPUCommandEncoder{device: d, cb: cb}, nil
}

// DestroyDevice destroys this device and releases all resources.
func (d *GPUDevice) DestroyDevice() {
	d.LogicalDevice.WaitIdle()
}

// =========================================================================
// GPUAdapter -- physical device wrapper
// =========================================================================

// GPUAdapter represents a physical GPU in WebGPU.
type GPUAdapter struct {
	physical *cr.PhysicalDevice
	Features map[string]bool
	Limits   GPUAdapterLimits
}

// Name returns the adapter name.
func (a *GPUAdapter) Name() string { return a.physical.Name() }

// RequestDevice requests a device from this adapter.
func (a *GPUAdapter) RequestDevice() (*GPUDevice, error) {
	return NewGPUDevice(a.physical.Vendor())
}

// =========================================================================
// GPU -- the top-level WebGPU entry point
// =========================================================================

// GPU is the WebGPU entry point -- like navigator.gpu in browsers.
type GPU struct {
	instance        *cr.RuntimeInstance
	physicalDevices []*cr.PhysicalDevice
}

// NewGPU creates the WebGPU entry point.
func NewGPU() (*GPU, error) {
	instance := cr.NewRuntimeInstance(nil)
	devices := instance.EnumeratePhysicalDevices()
	if len(devices) == 0 {
		return nil, fmt.Errorf("no GPU adapters available")
	}
	return &GPU{instance: instance, physicalDevices: devices}, nil
}

// RequestAdapter requests a GPU adapter.
func (g *GPU) RequestAdapter(options *GPURequestAdapterOptions) (*GPUAdapter, error) {
	if len(g.physicalDevices) == 0 {
		return nil, fmt.Errorf("no GPU adapters available")
	}

	// Pick based on power preference
	if options != nil && options.PowerPreference == "low-power" {
		for _, pd := range g.physicalDevices {
			if pd.MemProperties().IsUnified {
				return &GPUAdapter{
					physical: pd,
					Features: map[string]bool{"compute": true},
					Limits: GPUAdapterLimits{
						MaxBufferSize:            2 * 1024 * 1024 * 1024,
						MaxComputeWorkgroupSizeX: 1024,
					},
				}, nil
			}
		}
	}

	return &GPUAdapter{
		physical: g.physicalDevices[0],
		Features: map[string]bool{"compute": true},
		Limits: GPUAdapterLimits{
			MaxBufferSize:            2 * 1024 * 1024 * 1024,
			MaxComputeWorkgroupSizeX: 1024,
		},
	}, nil
}
