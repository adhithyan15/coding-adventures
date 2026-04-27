package vendorapisimulators

// Metal Runtime Simulator -- Apple's unified memory GPU programming model.
//
// # What is Metal?
//
// Metal is Apple's GPU API, designed exclusively for Apple hardware (macOS,
// iOS, iPadOS, tvOS). Its key innovation is unified memory -- on Apple Silicon
// (M1/M2/M3/M4), the CPU and GPU share the same physical RAM. This eliminates
// the host-to-device copies that CUDA and OpenCL require.
//
// # The Command Encoder Model
//
// Metal uses a distinctive pattern for recording GPU commands:
//
//  1. Get a command buffer from the command queue
//  2. Create a command encoder (compute, blit, render)
//  3. Record commands into the encoder
//  4. End the encoder
//  5. Commit the command buffer
//
// The encoder adds a layer of scoping that Vulkan does not have:
//
//	Vulkan: cb.Begin() -> CmdBindPipeline() -> CmdDispatch() -> cb.End()
//	Metal:  cb -> encoder = cb.MakeComputeCommandEncoder()
//	              encoder.SetComputePipelineState(pso)
//	              encoder.DispatchThreadgroups(...)
//	              encoder.EndEncoding()
//	        cb.Commit()
//
// # Unified Memory
//
// On Apple Silicon, all memory is both CPU-accessible and GPU-accessible:
//
//	CUDA:   cudaMalloc -> device-only, need cudaMemcpy to access from CPU
//	Metal:  MakeBuffer -> unified, buffer.Contents() gives CPU access directly

import (
	"fmt"

	cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// Suppress unused import warning for gpucore.
var _ = gpucore.Instruction{}

// =========================================================================
// Metal-specific types
// =========================================================================

// MTLSize is grid/threadgroup dimensions in Metal.
//
// Metal uses (width, height, depth) instead of (x, y, z). Same concept,
// different naming -- Apple convention for consistency with their graphics API.
type MTLSize struct {
	Width, Height, Depth int
}

// NewMTLSize creates a new MTLSize.
func NewMTLSize(w, h, d int) MTLSize {
	return MTLSize{Width: w, Height: h, Depth: d}
}

// MTLResourceOptions is the Metal storage mode options for buffers.
type MTLResourceOptions int

const (
	// MTLResourceStorageModeShared means CPU + GPU access (default on Apple Silicon).
	MTLResourceStorageModeShared MTLResourceOptions = iota
	// MTLResourceStorageModePrivate means GPU-only access, fastest for GPU-only buffers.
	MTLResourceStorageModePrivate
	// MTLResourceStorageModeManaged means CPU + GPU with explicit synchronization (macOS only).
	MTLResourceStorageModeManaged
)

// MTLCommandBufferStatus is the status of a Metal command buffer.
type MTLCommandBufferStatus int

const (
	MTLCommandBufferStatusNotEnqueued MTLCommandBufferStatus = iota
	MTLCommandBufferStatusEnqueued
	MTLCommandBufferStatusCommitted
	MTLCommandBufferStatusScheduled
	MTLCommandBufferStatusCompleted
	MTLCommandBufferStatusError
)

// =========================================================================
// MTLBuffer -- unified memory buffer
// =========================================================================

// MTLBuffer is a Metal buffer -- always accessible from both CPU and GPU.
//
// Because Apple Silicon uses unified memory, you can:
//
//	buf := device.MakeBuffer(1024, MTLResourceStorageModeShared)
//	buf.WriteBytes(data, 0)           // CPU writes directly
//	// ... GPU computes on buf ...
//	result := buf.Contents()          // CPU reads directly
//
// No staging buffers, no memcpy, no map/unmap ceremony.
type MTLBuffer struct {
	buffer *cr.Buffer
	mm     *cr.MemoryManager
	length int
}

// Length returns the buffer size in bytes.
func (b *MTLBuffer) Length() int { return b.length }

// Contents returns CPU-accessible view of the buffer contents.
//
// In real Metal, this returns a raw pointer to the shared memory.
func (b *MTLBuffer) Contents() ([]byte, error) {
	if err := b.mm.Invalidate(b.buffer, 0, 0); err != nil {
		return nil, err
	}
	return b.mm.GetBufferData(b.buffer.BufferID), nil
}

// WriteBytes writes bytes to the buffer from CPU side.
func (b *MTLBuffer) WriteBytes(data []byte, offset int) error {
	mapped, err := b.mm.Map(b.buffer)
	if err != nil {
		return err
	}
	if err := mapped.Write(offset, data); err != nil {
		return err
	}
	return b.mm.Unmap(b.buffer)
}

// =========================================================================
// MTLFunction and MTLLibrary -- shader management
// =========================================================================

// MTLFunction is a Metal shader function extracted from a library.
type MTLFunction struct {
	name string
	code []gpucore.Instruction
}

// Name returns the function name.
func (f *MTLFunction) Name() string { return f.name }

// MTLLibrary is a Metal shader library -- a collection of compiled functions.
type MTLLibrary struct {
	source    string
	functions map[string][]gpucore.Instruction
}

// MakeFunction extracts a function from the library by name.
func (l *MTLLibrary) MakeFunction(name string) *MTLFunction {
	code := l.functions[name]
	return &MTLFunction{name: name, code: code}
}

// =========================================================================
// MTLComputePipelineState -- compiled compute pipeline
// =========================================================================

// MTLComputePipelineState is a compiled Metal compute pipeline state.
type MTLComputePipelineState struct {
	function *MTLFunction
	device   *cr.LogicalDevice
	pipeline *cr.Pipeline
}

// MaxTotalThreadsPerThreadgroup returns max threads per threadgroup for this pipeline.
func (pso *MTLComputePipelineState) MaxTotalThreadsPerThreadgroup() int {
	return 1024
}

// =========================================================================
// MTLComputeCommandEncoder -- records compute commands
// =========================================================================

// MTLComputeCommandEncoder records compute commands in Metal's encoder pattern.
//
// Instead of recording commands directly into a command buffer (Vulkan style),
// Metal uses typed encoders that scope commands by type:
//
//	encoder := commandBuffer.MakeComputeCommandEncoder()
//	encoder.SetComputePipelineState(pso)
//	encoder.SetBuffer(bufX, 0, 0)
//	encoder.SetBuffer(bufY, 0, 1)
//	encoder.DispatchThreadgroups(groups, threadsPerGroup)
//	encoder.EndEncoding()
type MTLComputeCommandEncoder struct {
	commandBuffer *MTLCommandBuffer
	pipelineState *MTLComputePipelineState
	buffers       map[int]*MTLBuffer
	pushData      map[int][]byte
	ended         bool
}

// SetComputePipelineState sets which compute pipeline to use.
func (e *MTLComputeCommandEncoder) SetComputePipelineState(pso *MTLComputePipelineState) {
	e.pipelineState = pso
}

// SetBuffer binds a buffer to an argument index.
func (e *MTLComputeCommandEncoder) SetBuffer(buffer *MTLBuffer, offset int, index int) {
	e.buffers[index] = buffer
}

// SetBytes sets inline bytes as a kernel argument (push constants).
func (e *MTLComputeCommandEncoder) SetBytes(data []byte, index int) {
	e.pushData[index] = data
}

// DispatchThreadgroups dispatches a compute kernel with explicit threadgroup count.
func (e *MTLComputeCommandEncoder) DispatchThreadgroups(
	threadgroupsPerGrid MTLSize,
	threadsPerThreadgroup MTLSize,
) error {
	if e.pipelineState == nil {
		return fmt.Errorf("no compute pipeline state set")
	}

	cb := e.commandBuffer.cb
	device := e.commandBuffer.device

	// Create a fresh pipeline with the correct local size
	pso := e.pipelineState
	shader := device.CreateShaderModule(cr.ShaderModuleOptions{
		Code: pso.function.code,
		LocalSize: [3]int{
			threadsPerThreadgroup.Width,
			threadsPerThreadgroup.Height,
			threadsPerThreadgroup.Depth,
		},
	})

	// Build descriptor set from bound buffers
	sortedBufIndices := sortedMTLBufferKeys(e.buffers)
	bindings := make([]cr.DescriptorBinding, len(sortedBufIndices))
	for j, i := range sortedBufIndices {
		bindings[j] = cr.DescriptorBinding{Binding: i, Type: "storage", Count: 1}
	}
	dsLayout := device.CreateDescriptorSetLayout(bindings)
	plLayout := device.CreatePipelineLayout([]*cr.DescriptorSetLayout{dsLayout}, 0)
	pipeline := device.CreateComputePipeline(shader, plLayout)

	ds := device.CreateDescriptorSet(dsLayout)
	for _, i := range sortedBufIndices {
		if err := ds.Write(i, e.buffers[i].buffer); err != nil {
			return err
		}
	}

	// Record into the command buffer
	if err := cb.CmdBindPipeline(pipeline); err != nil {
		return err
	}
	if err := cb.CmdBindDescriptorSet(ds); err != nil {
		return err
	}
	return cb.CmdDispatch(
		threadgroupsPerGrid.Width,
		threadgroupsPerGrid.Height,
		threadgroupsPerGrid.Depth,
	)
}

// DispatchThreads dispatches with total thread count (Metal calculates grid).
func (e *MTLComputeCommandEncoder) DispatchThreads(
	threadsPerGrid MTLSize,
	threadsPerThreadgroup MTLSize,
) error {
	groups := MTLSize{
		Width:  max(1, (threadsPerGrid.Width+threadsPerThreadgroup.Width-1)/threadsPerThreadgroup.Width),
		Height: max(1, (threadsPerGrid.Height+threadsPerThreadgroup.Height-1)/threadsPerThreadgroup.Height),
		Depth:  max(1, (threadsPerGrid.Depth+threadsPerThreadgroup.Depth-1)/threadsPerThreadgroup.Depth),
	}
	return e.DispatchThreadgroups(groups, threadsPerThreadgroup)
}

// EndEncoding ends recording into this encoder.
func (e *MTLComputeCommandEncoder) EndEncoding() {
	e.ended = true
}

// =========================================================================
// MTLBlitCommandEncoder -- records data transfer commands
// =========================================================================

// MTLBlitCommandEncoder records copy/fill operations.
//
// "Blit" stands for "block image transfer" -- a term from early computer
// graphics for bulk memory copies.
type MTLBlitCommandEncoder struct {
	commandBuffer *MTLCommandBuffer
	ended         bool
}

// CopyFromBuffer copies data between buffers.
func (e *MTLBlitCommandEncoder) CopyFromBuffer(
	src *MTLBuffer, srcOffset int,
	toBuffer *MTLBuffer, dstOffset int,
	size int,
) error {
	cb := e.commandBuffer.cb
	return cb.CmdCopyBuffer(src.buffer, toBuffer.buffer, size, srcOffset, dstOffset)
}

// FillBuffer fills a buffer region with a byte value.
func (e *MTLBlitCommandEncoder) FillBuffer(buffer *MTLBuffer, start, length, value int) error {
	cb := e.commandBuffer.cb
	return cb.CmdFillBuffer(buffer.buffer, value, start, length)
}

// EndEncoding ends recording into this blit encoder.
func (e *MTLBlitCommandEncoder) EndEncoding() {
	e.ended = true
}

// =========================================================================
// MTLCommandBuffer -- wraps Layer 5 CommandBuffer with encoder model
// =========================================================================

// MTLCommandBuffer records and submits GPU work using the encoder model.
type MTLCommandBuffer struct {
	queue  *MTLCommandQueue
	device *cr.LogicalDevice
	cb     *cr.CommandBuffer
	fence  *cr.Fence
	status MTLCommandBufferStatus
}

// Status returns the current command buffer status.
func (mcb *MTLCommandBuffer) Status() MTLCommandBufferStatus {
	return mcb.status
}

// MakeComputeCommandEncoder creates a compute command encoder.
func (mcb *MTLCommandBuffer) MakeComputeCommandEncoder() *MTLComputeCommandEncoder {
	return &MTLComputeCommandEncoder{
		commandBuffer: mcb,
		buffers:       make(map[int]*MTLBuffer),
		pushData:      make(map[int][]byte),
	}
}

// MakeBlitCommandEncoder creates a blit (copy/fill) command encoder.
func (mcb *MTLCommandBuffer) MakeBlitCommandEncoder() *MTLBlitCommandEncoder {
	return &MTLBlitCommandEncoder{commandBuffer: mcb}
}

// Commit submits this command buffer for execution.
func (mcb *MTLCommandBuffer) Commit() error {
	if err := mcb.cb.End(); err != nil {
		return err
	}
	mcb.status = MTLCommandBufferStatusCommitted
	_, err := mcb.queue.queue.Submit(
		[]*cr.CommandBuffer{mcb.cb},
		&cr.SubmitOptions{Fence: mcb.fence},
	)
	if err != nil {
		mcb.status = MTLCommandBufferStatusError
		return err
	}
	mcb.status = MTLCommandBufferStatusCompleted
	return nil
}

// WaitUntilCompleted blocks until the command buffer finishes execution.
func (mcb *MTLCommandBuffer) WaitUntilCompleted() {
	mcb.fence.Wait(nil)
}

// =========================================================================
// MTLCommandQueue -- creates command buffers
// =========================================================================

// MTLCommandQueue creates command buffers for submission.
type MTLCommandQueue struct {
	device *MTLDevice
	queue  *cr.CommandQueue
}

// MakeCommandBuffer creates a new command buffer for this queue.
func (q *MTLCommandQueue) MakeCommandBuffer() (*MTLCommandBuffer, error) {
	device := q.device.LogicalDevice
	cb := device.CreateCommandBuffer()
	if err := cb.Begin(); err != nil {
		return nil, err
	}
	fence := device.CreateFence(false)
	return &MTLCommandBuffer{
		queue:  q,
		device: device,
		cb:     cb,
		fence:  fence,
		status: MTLCommandBufferStatusNotEnqueued,
	}, nil
}

// =========================================================================
// MTLDevice -- the main Metal device object
// =========================================================================

// MTLDevice is the main entry point for Metal programming.
//
// # Apple's Simplified Model
//
// In Vulkan, you have PhysicalDevice (read-only) and LogicalDevice (usable).
// In Metal, there is just MTLDevice -- it is both.
//
// Metal always uses unified memory. All buffers are CPU-accessible by default.
type MTLDevice struct {
	*BaseVendorSimulator
}

// NewMTLDevice creates a Metal device, preferring Apple hardware.
func NewMTLDevice() (*MTLDevice, error) {
	base, err := InitBase(nil, "apple")
	if err != nil {
		return nil, fmt.Errorf("failed to initialize Metal device: %w", err)
	}
	return &MTLDevice{BaseVendorSimulator: base}, nil
}

// Name returns the device name.
func (d *MTLDevice) Name() string {
	return d.PhysicalDevice.Name()
}

// MakeCommandQueue creates a command queue for this device.
func (d *MTLDevice) MakeCommandQueue() *MTLCommandQueue {
	return &MTLCommandQueue{device: d, queue: d.ComputeQueue}
}

// MakeBuffer allocates a buffer on the device.
//
// All Metal buffers use unified memory by default (storageModeShared).
func (d *MTLDevice) MakeBuffer(length int, options MTLResourceOptions) (*MTLBuffer, error) {
	buf, err := d.MemoryManager.Allocate(length, DefaultMemType(), DefaultUsage())
	if err != nil {
		return nil, err
	}
	return &MTLBuffer{buffer: buf, mm: d.MemoryManager, length: length}, nil
}

// MakeLibrary creates a shader library from source code.
func (d *MTLDevice) MakeLibrary(source string) *MTLLibrary {
	return &MTLLibrary{source: source, functions: make(map[string][]gpucore.Instruction)}
}

// MakeComputePipelineState creates a compute pipeline state from a shader function.
func (d *MTLDevice) MakeComputePipelineState(function *MTLFunction) *MTLComputePipelineState {
	device := d.LogicalDevice
	shader := device.CreateShaderModule(cr.ShaderModuleOptions{Code: function.code})
	dsLayout := device.CreateDescriptorSetLayout(nil)
	plLayout := device.CreatePipelineLayout([]*cr.DescriptorSetLayout{dsLayout}, 0)
	pipeline := device.CreateComputePipeline(shader, plLayout)
	return &MTLComputePipelineState{
		function: function,
		device:   device,
		pipeline: pipeline,
	}
}

// sortedMTLBufferKeys returns sorted keys of a map[int]*MTLBuffer.
func sortedMTLBufferKeys(m map[int]*MTLBuffer) []int {
	keys := make([]int, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	for i := 1; i < len(keys); i++ {
		for j := i; j > 0 && keys[j-1] > keys[j]; j-- {
			keys[j-1], keys[j] = keys[j], keys[j-1]
		}
	}
	return keys
}
