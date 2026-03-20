package vendorapisimulators

// OpenCL Runtime Simulator -- cross-platform "portable compute" model.
//
// # What is OpenCL?
//
// OpenCL (Open Computing Language) is the Khronos Group's cross-platform
// compute API. Unlike CUDA (NVIDIA only), OpenCL runs on any vendor's GPU,
// and even on CPUs and FPGAs. The tradeoff is more boilerplate -- you must
// explicitly manage platforms, devices, contexts, and command queues.
//
// # The OpenCL Object Hierarchy
//
//	CLPlatform          "Which vendor's implementation?"
//	    |-- CLDevice    "Which specific GPU/CPU?"
//	CLContext            "A group of devices I want to use together"
//	    |-- CLBuffer     "Memory on one of the context's devices"
//	    |-- CLProgram    "Source code, not yet compiled"
//	    |   |-- CLKernel "Compiled function, ready to dispatch"
//	    |-- CLCommandQueue "Where I enqueue operations"
//	            |-- CLEvent "Dependency token for operation ordering"
//
// # Event-Based Dependencies
//
// OpenCL's most distinctive feature is its event model. Every enqueue
// operation returns a CLEvent. You can pass event lists to subsequent
// operations to create dependency chains:
//
//	ev1 := queue.EnqueueWriteBuffer(buf_x, data_x)
//	ev2 := queue.EnqueueWriteBuffer(buf_y, data_y)
//	ev3 := queue.EnqueueNDRangeKernel(kernel, waitList=[ev1, ev2])
//	ev4 := queue.EnqueueReadBuffer(buf_y, waitList=[ev3])
//
// This is more flexible than CUDA's stream model because dependencies
// can form arbitrary DAGs, not just linear sequences.

import (
	"fmt"

	cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// Suppress unused import warning for gpucore (used in CLKernel.Code type).
var _ []gpucore.Instruction

// =========================================================================
// OpenCL enums and types
// =========================================================================

// CLDeviceType is an OpenCL device type for filtering during discovery.
type CLDeviceType int

const (
	CLDeviceTypeGPU CLDeviceType = iota
	CLDeviceTypeCPU
	CLDeviceTypeAccelerator
	CLDeviceTypeAll
)

// CLMemFlags are OpenCL memory flags -- simpler than Vulkan's memory types.
type CLMemFlags int

const (
	CLMemReadWrite   CLMemFlags = 1 << iota // GPU can read and write
	CLMemReadOnly                            // GPU can only read
	CLMemWriteOnly                           // GPU can only write
	CLMemCopyHostPtr                         // Initialize from host data
	CLMemUseHostPtr                          // Use the host pointer directly
	CLMemAllocHostPtr                        // Allocate in host-visible memory
)

// CLBuildStatus is the build status of a CLProgram.
type CLBuildStatus int

const (
	CLBuildSuccess CLBuildStatus = iota
	CLBuildError
	CLBuildInProgress
	CLBuildNone
)

// CLEventStatus is the status of an OpenCL event.
type CLEventStatus int

const (
	CLEventQueued CLEventStatus = iota
	CLEventSubmitted
	CLEventRunning
	CLEventComplete
)

// CLDeviceInfo is a device info parameter ID for CLDevice.GetInfo().
type CLDeviceInfo int

const (
	CLDeviceInfoName CLDeviceInfo = iota
	CLDeviceInfoType
	CLDeviceInfoMaxComputeUnits
	CLDeviceInfoMaxWorkGroupSize
	CLDeviceInfoGlobalMemSize
)

// =========================================================================
// CLEvent -- dependency token
// =========================================================================

// CLEvent is an OpenCL event -- a dependency token for operation ordering.
//
// Every enqueue operation returns a CLEvent. You can:
//   - Wait on it (blocking the CPU)
//   - Pass it in a wait list to another operation (GPU-side dependency)
//   - Query its status
type CLEvent struct {
	fence *cr.Fence
}

// Wait blocks until this event completes.
func (e *CLEvent) Wait() {
	e.fence.Wait(nil)
}

// Status returns the current status of this event.
func (e *CLEvent) Status() CLEventStatus {
	if e.fence.Signaled() {
		return CLEventComplete
	}
	return CLEventQueued
}

// =========================================================================
// CLDevice -- wraps PhysicalDevice
// =========================================================================

// CLDevice is an OpenCL device -- a specific piece of hardware.
type CLDevice struct {
	physical *cr.PhysicalDevice
}

// Name returns the device name.
func (d *CLDevice) Name() string { return d.physical.Name() }

// DeviceType returns the device type (GPU, CPU, etc.).
func (d *CLDevice) DeviceType() CLDeviceType {
	dt := d.physical.DeviceType().String()
	switch dt {
	case "gpu":
		return CLDeviceTypeGPU
	case "tpu", "npu":
		return CLDeviceTypeAccelerator
	default:
		return CLDeviceTypeGPU
	}
}

// MaxComputeUnits returns the number of compute units.
func (d *CLDevice) MaxComputeUnits() int { return 4 }

// MaxWorkGroupSize returns the maximum work items per work group.
func (d *CLDevice) MaxWorkGroupSize() int {
	return d.physical.Limits().MaxWorkgroupSize[0]
}

// GlobalMemSize returns total global memory in bytes.
func (d *CLDevice) GlobalMemSize() int {
	total := 0
	for _, h := range d.physical.MemProperties().Heaps {
		total += h.Size
	}
	return total
}

// GetInfo queries device information by parameter ID.
func (d *CLDevice) GetInfo(param CLDeviceInfo) interface{} {
	switch param {
	case CLDeviceInfoName:
		return d.Name()
	case CLDeviceInfoType:
		return d.DeviceType()
	case CLDeviceInfoMaxComputeUnits:
		return d.MaxComputeUnits()
	case CLDeviceInfoMaxWorkGroupSize:
		return d.MaxWorkGroupSize()
	case CLDeviceInfoGlobalMemSize:
		return d.GlobalMemSize()
	default:
		return nil
	}
}

// =========================================================================
// CLBuffer -- wraps Buffer
// =========================================================================

// CLBuffer is an OpenCL buffer -- memory allocated on a device.
type CLBuffer struct {
	buffer *cr.Buffer
	size   int
	flags  CLMemFlags
}

// Size returns the buffer size in bytes.
func (b *CLBuffer) Size() int { return b.size }

// Flags returns the memory flags.
func (b *CLBuffer) Flags() CLMemFlags { return b.flags }

// =========================================================================
// CLKernel -- a compiled kernel function
// =========================================================================

// CLKernel is an OpenCL kernel -- a compiled function extracted from a CLProgram.
//
// In OpenCL, kernel arguments are set one at a time with SetArg().
type CLKernel struct {
	name string
	code []gpucore.Instruction
	args map[int]interface{} // index -> CLBuffer, int, float, or []byte
}

// Name returns the kernel function name.
func (k *CLKernel) Name() string { return k.name }

// SetArg sets a kernel argument at the given index.
//
// In OpenCL, arguments are set individually before enqueueing:
//
//	kernel.SetArg(0, bufX)    // binding 0 = input X
//	kernel.SetArg(1, bufY)    // binding 1 = output Y
func (k *CLKernel) SetArg(index int, value interface{}) {
	k.args[index] = value
}

// =========================================================================
// CLProgram -- source code + compilation
// =========================================================================

// CLProgram is an OpenCL program -- source code that can be compiled for a device.
//
// OpenCL uses runtime compilation: you provide kernel source as a string,
// call Build(), and the OpenCL implementation compiles it for the target device.
type CLProgram struct {
	source      string
	context     *CLContext
	buildStatus CLBuildStatus
	kernels     map[string][]gpucore.Instruction
}

// BuildStatus returns the current build status.
func (p *CLProgram) BuildStatus() CLBuildStatus { return p.buildStatus }

// Build compiles the program for the target device(s).
func (p *CLProgram) Build(devices []*CLDevice, options string) {
	p.buildStatus = CLBuildSuccess
}

// CreateKernel extracts a kernel function from the compiled program.
func (p *CLProgram) CreateKernel(name string) (*CLKernel, error) {
	if p.buildStatus != CLBuildSuccess {
		return nil, fmt.Errorf("program not built (status: %d). Call Build() first", p.buildStatus)
	}
	code := p.kernels[name]
	return &CLKernel{
		name: name,
		code: code,
		args: make(map[int]interface{}),
	}, nil
}

// =========================================================================
// CLCommandQueue -- enqueue operations with event dependencies
// =========================================================================

// CLCommandQueue is where operations are enqueued in OpenCL.
//
// Every operation returns a CLEvent for dependency tracking.
type CLCommandQueue struct {
	context *CLContext
	device  *CLDevice
}

// EnqueueNDRangeKernel enqueues a kernel for execution (clEnqueueNDRangeKernel).
//
// globalSize specifies total work items. localSize can be nil for auto-select.
func (q *CLCommandQueue) EnqueueNDRangeKernel(
	kernel *CLKernel,
	globalSize []int,
	localSize []int,
	waitList []*CLEvent,
) (*CLEvent, error) {
	// Wait for dependency events
	for _, event := range waitList {
		event.Wait()
	}

	device := q.context.LogicalDevice

	// Determine local size (workgroup size)
	local := [3]int{32, 1, 1}
	if localSize != nil {
		local[0] = localSize[0]
		if len(localSize) > 1 {
			local[1] = localSize[1]
		}
		if len(localSize) > 2 {
			local[2] = localSize[2]
		}
	}

	// Calculate grid dimensions (number of workgroups)
	gridX := max(1, (globalSize[0]+local[0]-1)/local[0])
	gridY := 1
	if len(globalSize) > 1 {
		gridY = max(1, (globalSize[1]+local[1]-1)/local[1])
	}
	gridZ := 1
	if len(globalSize) > 2 {
		gridZ = max(1, (globalSize[2]+local[2]-1)/local[2])
	}

	// Create shader module from kernel code
	shader := device.CreateShaderModule(cr.ShaderModuleOptions{
		Code:      kernel.code,
		LocalSize: local,
	})

	// Build descriptor set from kernel arguments
	bufferArgs := map[int]*CLBuffer{}
	for i, arg := range kernel.args {
		if buf, ok := arg.(*CLBuffer); ok {
			bufferArgs[i] = buf
		}
	}
	bindings := make([]cr.DescriptorBinding, 0, len(bufferArgs))
	sortedIndices := sortedKeys(bufferArgs)
	for _, i := range sortedIndices {
		bindings = append(bindings, cr.DescriptorBinding{Binding: i, Type: "storage", Count: 1})
	}
	dsLayout := device.CreateDescriptorSetLayout(bindings)
	plLayout := device.CreatePipelineLayout([]*cr.DescriptorSetLayout{dsLayout}, 0)
	pipeline := device.CreateComputePipeline(shader, plLayout)

	ds := device.CreateDescriptorSet(dsLayout)
	for _, i := range sortedIndices {
		if err := ds.Write(i, bufferArgs[i].buffer); err != nil {
			return nil, err
		}
	}

	// Record and submit
	fence := device.CreateFence(false)
	cb := device.CreateCommandBuffer()
	if err := cb.Begin(); err != nil {
		return nil, err
	}
	if err := cb.CmdBindPipeline(pipeline); err != nil {
		return nil, err
	}
	if err := cb.CmdBindDescriptorSet(ds); err != nil {
		return nil, err
	}
	if err := cb.CmdDispatch(gridX, gridY, gridZ); err != nil {
		return nil, err
	}
	if err := cb.End(); err != nil {
		return nil, err
	}

	queue := q.context.ComputeQueue
	_, err := queue.Submit([]*cr.CommandBuffer{cb}, &cr.SubmitOptions{Fence: fence})
	if err != nil {
		return nil, err
	}
	fence.Wait(nil)

	return &CLEvent{fence: fence}, nil
}

// EnqueueWriteBuffer writes host data to a device buffer (clEnqueueWriteBuffer).
func (q *CLCommandQueue) EnqueueWriteBuffer(
	buffer *CLBuffer,
	offset int,
	size int,
	hostPtr []byte,
	waitList []*CLEvent,
) (*CLEvent, error) {
	for _, event := range waitList {
		event.Wait()
	}

	mm := q.context.MemoryManager
	mapped, err := mm.Map(buffer.buffer)
	if err != nil {
		return nil, err
	}
	copyLen := size
	if copyLen > len(hostPtr) {
		copyLen = len(hostPtr)
	}
	if err := mapped.Write(offset, hostPtr[:copyLen]); err != nil {
		return nil, err
	}
	if err := mm.Unmap(buffer.buffer); err != nil {
		return nil, err
	}

	fence := q.context.LogicalDevice.CreateFence(true)
	return &CLEvent{fence: fence}, nil
}

// EnqueueReadBuffer reads device buffer data to host memory (clEnqueueReadBuffer).
func (q *CLCommandQueue) EnqueueReadBuffer(
	buffer *CLBuffer,
	offset int,
	size int,
	hostPtr []byte,
	waitList []*CLEvent,
) (*CLEvent, error) {
	for _, event := range waitList {
		event.Wait()
	}

	mm := q.context.MemoryManager
	if err := mm.Invalidate(buffer.buffer, 0, 0); err != nil {
		return nil, err
	}
	mapped, err := mm.Map(buffer.buffer)
	if err != nil {
		return nil, err
	}
	data, err := mapped.Read(offset, size)
	if err != nil {
		return nil, err
	}
	if err := mm.Unmap(buffer.buffer); err != nil {
		return nil, err
	}
	copy(hostPtr[:size], data)

	fence := q.context.LogicalDevice.CreateFence(true)
	return &CLEvent{fence: fence}, nil
}

// EnqueueCopyBuffer copies between two device buffers (clEnqueueCopyBuffer).
func (q *CLCommandQueue) EnqueueCopyBuffer(
	src, dst *CLBuffer,
	size int,
	waitList []*CLEvent,
) (*CLEvent, error) {
	for _, event := range waitList {
		event.Wait()
	}

	device := q.context.LogicalDevice
	fence := device.CreateFence(false)
	cb := device.CreateCommandBuffer()
	if err := cb.Begin(); err != nil {
		return nil, err
	}
	if err := cb.CmdCopyBuffer(src.buffer, dst.buffer, size, 0, 0); err != nil {
		return nil, err
	}
	if err := cb.End(); err != nil {
		return nil, err
	}
	_, err := q.context.ComputeQueue.Submit([]*cr.CommandBuffer{cb}, &cr.SubmitOptions{Fence: fence})
	if err != nil {
		return nil, err
	}
	fence.Wait(nil)
	return &CLEvent{fence: fence}, nil
}

// EnqueueFillBuffer fills a buffer with a pattern (clEnqueueFillBuffer).
func (q *CLCommandQueue) EnqueueFillBuffer(
	buffer *CLBuffer,
	pattern []byte,
	offset, size int,
) (*CLEvent, error) {
	device := q.context.LogicalDevice
	fence := device.CreateFence(false)
	cb := device.CreateCommandBuffer()
	if err := cb.Begin(); err != nil {
		return nil, err
	}
	value := 0
	if len(pattern) > 0 {
		value = int(pattern[0])
	}
	if err := cb.CmdFillBuffer(buffer.buffer, value, offset, size); err != nil {
		return nil, err
	}
	if err := cb.End(); err != nil {
		return nil, err
	}
	_, err := q.context.ComputeQueue.Submit([]*cr.CommandBuffer{cb}, &cr.SubmitOptions{Fence: fence})
	if err != nil {
		return nil, err
	}
	fence.Wait(nil)
	return &CLEvent{fence: fence}, nil
}

// Finish blocks until all enqueued operations complete (clFinish).
func (q *CLCommandQueue) Finish() {
	q.context.LogicalDevice.WaitIdle()
}

// Flush ensures all enqueued operations are submitted (clFlush).
// In our synchronous simulator, this is a no-op.
func (q *CLCommandQueue) Flush() {}

// =========================================================================
// CLContext -- the OpenCL execution context
// =========================================================================

// CLContext is an OpenCL context -- groups devices and manages shared resources.
type CLContext struct {
	*BaseVendorSimulator
	devices []*CLDevice
}

// NewCLContext creates an OpenCL context.
func NewCLContext(devices []*CLDevice) (*CLContext, error) {
	var vendorHint string
	if len(devices) > 0 {
		vendorHint = devices[0].physical.Vendor()
	}
	base, err := InitBase(nil, vendorHint)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize OpenCL context: %w", err)
	}

	ctx := &CLContext{BaseVendorSimulator: base}
	if len(devices) > 0 {
		ctx.devices = devices
	} else {
		for _, pd := range base.PhysicalDevices {
			ctx.devices = append(ctx.devices, &CLDevice{physical: pd})
		}
	}
	return ctx, nil
}

// Devices returns all devices in this context.
func (c *CLContext) Devices() []*CLDevice { return c.devices }

// CreateBuffer creates a device buffer (clCreateBuffer).
func (c *CLContext) CreateBuffer(flags CLMemFlags, size int, hostPtr []byte) (*CLBuffer, error) {
	buf, err := c.MemoryManager.Allocate(size, DefaultMemType(), DefaultUsage())
	if err != nil {
		return nil, err
	}
	clBuf := &CLBuffer{buffer: buf, size: size, flags: flags}

	// If COPY_HOST_PTR, write the initial data
	if hostPtr != nil && flags&CLMemCopyHostPtr != 0 {
		mapped, err := c.MemoryManager.Map(buf)
		if err != nil {
			return nil, err
		}
		copyLen := size
		if copyLen > len(hostPtr) {
			copyLen = len(hostPtr)
		}
		if err := mapped.Write(0, hostPtr[:copyLen]); err != nil {
			return nil, err
		}
		if err := c.MemoryManager.Unmap(buf); err != nil {
			return nil, err
		}
	}

	return clBuf, nil
}

// CreateProgramWithSource creates a program from source code (clCreateProgramWithSource).
func (c *CLContext) CreateProgramWithSource(source string) *CLProgram {
	return &CLProgram{
		source:      source,
		context:     c,
		buildStatus: CLBuildNone,
		kernels:     make(map[string][]gpucore.Instruction),
	}
}

// CreateCommandQueue creates a command queue for a device (clCreateCommandQueue).
func (c *CLContext) CreateCommandQueue(device *CLDevice) *CLCommandQueue {
	dev := device
	if dev == nil && len(c.devices) > 0 {
		dev = c.devices[0]
	}
	return &CLCommandQueue{context: c, device: dev}
}

// =========================================================================
// CLPlatform -- the top-level discovery object
// =========================================================================

// CLPlatform is an OpenCL platform -- represents a vendor's OpenCL implementation.
type CLPlatform struct {
	name    string
	vendor  string
	version string
	base    *BaseVendorSimulator
}

// NewCLPlatform creates a new platform by discovering devices.
func NewCLPlatform() (*CLPlatform, error) {
	base, err := InitBase(nil, "")
	if err != nil {
		return nil, err
	}
	return &CLPlatform{
		name:    "Coding Adventures Compute Platform",
		vendor:  "Coding Adventures",
		version: "OpenCL 3.0",
		base:    base,
	}, nil
}

// GetPlatforms returns available OpenCL platforms.
func GetPlatforms() ([]*CLPlatform, error) {
	p, err := NewCLPlatform()
	if err != nil {
		return nil, err
	}
	return []*CLPlatform{p}, nil
}

// Name returns the platform name.
func (p *CLPlatform) Name() string { return p.name }

// Vendor returns the platform vendor.
func (p *CLPlatform) Vendor() string { return p.vendor }

// Version returns the platform version string.
func (p *CLPlatform) Version() string { return p.version }

// GetDevices returns devices of a specific type on this platform.
func (p *CLPlatform) GetDevices(deviceType CLDeviceType) []*CLDevice {
	var devices []*CLDevice
	for _, pd := range p.base.PhysicalDevices {
		dev := &CLDevice{physical: pd}
		if deviceType == CLDeviceTypeAll || dev.DeviceType() == deviceType {
			devices = append(devices, dev)
		}
	}
	return devices
}

// sortedKeys returns the sorted keys of a map[int]*CLBuffer.
func sortedKeys(m map[int]*CLBuffer) []int {
	keys := make([]int, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	// Simple insertion sort for small maps
	for i := 1; i < len(keys); i++ {
		for j := i; j > 0 && keys[j-1] > keys[j]; j-- {
			keys[j-1], keys[j] = keys[j], keys[j-1]
		}
	}
	return keys
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
