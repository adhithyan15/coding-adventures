package vendorapisimulators

// CUDA Runtime Simulator -- NVIDIA's "just launch it" GPU programming model.
//
// # What is CUDA?
//
// CUDA (Compute Unified Device Architecture) is NVIDIA's proprietary GPU
// computing platform. It is the most popular GPU programming API, used by
// PyTorch, TensorFlow, and virtually all ML research.
//
// CUDA's design philosophy is "make the common case easy." The common case:
//
//  1. Allocate memory on the GPU          --> cudaMalloc()
//  2. Copy data from CPU to GPU           --> cudaMemcpy(HostToDevice)
//  3. Launch a kernel                     --> kernel<<<grid, block>>>(args)
//  4. Copy results back                   --> cudaMemcpy(DeviceToHost)
//  5. Free memory                         --> cudaFree()
//
// Each of these is a single function call. Compare this to Vulkan, where
// launching a kernel requires creating a pipeline, descriptor set, command
// buffer, recording commands, submitting, and waiting.
//
// # How CUDA Hides Complexity
//
// When you write kernel<<<grid, block>>>(args) in CUDA, here is what
// happens internally (and what our simulator does):
//
//  1. Create a Pipeline from the kernel's code
//  2. Create a DescriptorSet and bind the argument buffers
//  3. Create a CommandBuffer
//  4. Record: bind_pipeline, bind_descriptor_set, dispatch
//  5. Submit the CommandBuffer to the default stream's queue
//  6. Wait for completion (synchronous in default stream)
//
// # Streams
//
// CUDA streams are independent execution queues. The default stream (stream 0)
// is synchronous. Additional streams can overlap:
//
//	Stream 0 (default):  [kernel A]--[kernel B]--[kernel C]
//	Stream 1:            --[upload]--[kernel D]--[download]
//
// # Memory Model
//
// CUDA simplifies memory into two main types:
//
//	cudaMalloc():        GPU-only memory (DEVICE_LOCAL in Layer 5)
//	cudaMallocManaged(): Unified memory accessible from both CPU and GPU

import (
	"fmt"

	cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// CUDA-specific types
// =========================================================================

// Dim3 is the classic CUDA grid/block dimension type.
//
// In real CUDA, dim3 is a struct with x, y, z fields. When you write
// kernel<<<dim3(4, 1, 1), dim3(64, 1, 1)>>>, you are saying:
// "Launch 4 blocks of 64 threads each, in 1D."
type Dim3 struct {
	X, Y, Z int
}

// NewDim3 creates a Dim3 with the given dimensions.
func NewDim3(x, y, z int) Dim3 {
	return Dim3{X: x, Y: y, Z: z}
}

// CUDAMemcpyKind identifies the direction of a CUDA memory copy.
//
// # The Four Copy Directions
//
//	HostToDevice:   CPU RAM -> GPU VRAM (upload)
//	DeviceToHost:   GPU VRAM -> CPU RAM (download)
//	DeviceToDevice: GPU VRAM -> GPU VRAM (on-device copy)
//	HostToHost:     CPU RAM -> CPU RAM (plain memcpy)
//
// In real CUDA, these map to different DMA engine configurations:
//   - HostToDevice uses the PCIe DMA engine (CPU->GPU direction)
//   - DeviceToHost uses the PCIe DMA engine (GPU->CPU direction)
//   - DeviceToDevice uses the internal GPU copy engine
//   - HostToHost uses plain CPU memcpy (no GPU involvement)
type CUDAMemcpyKind int

const (
	CUDAMemcpyHostToDevice CUDAMemcpyKind = iota
	CUDAMemcpyDeviceToHost
	CUDAMemcpyDeviceToDevice
	CUDAMemcpyHostToHost
)

// CUDADeviceProperties holds properties of a CUDA device, similar to cudaDeviceProp.
//
// In real CUDA, you query these with cudaGetDeviceProperties(). They tell you
// what the GPU can do -- how much memory, how many threads, what compute capability.
type CUDADeviceProperties struct {
	Name               string
	TotalGlobalMem     int
	SharedMemPerBlock  int
	MaxThreadsPerBlock int
	MaxGridSize        [3]int
	WarpSize           int
	ComputeCapability  [2]int
}

// CUDAKernel is a CUDA kernel -- compiled GPU code ready to launch.
//
// In real CUDA, kernels are C++ functions decorated with __global__.
// In our simulator, a kernel wraps a list of GPU instructions from
// the gpu-core package (Layer 9).
type CUDAKernel struct {
	Code []gpucore.Instruction
	Name string
}

// CUDADevicePtr is a CUDA device pointer -- a handle to GPU memory.
//
// In real CUDA, cudaMalloc() returns a void* pointer to device memory.
// You cannot dereference it on the CPU -- it is only valid on the GPU.
//
// In our simulator, CUDADevicePtr wraps a Layer 5 Buffer object and
// exposes its DeviceAddress and Size.
type CUDADevicePtr struct {
	Buffer        *cr.Buffer
	DeviceAddress int
	Size          int
}

// CUDAStream is a CUDA stream -- an independent execution queue.
//
// A stream is a sequence of GPU operations that execute in order.
// Operations in the same stream are guaranteed to execute sequentially.
// Operations in different streams MAY execute concurrently.
//
// The default stream (stream 0) has special semantics -- it synchronizes
// with all other streams. Our simulator models each stream as a separate
// Layer 5 CommandQueue.
type CUDAStream struct {
	queue        *cr.CommandQueue
	pendingFence *cr.Fence
}

// CUDAEvent is a CUDA event -- a timestamp marker in a stream.
//
// Events are used for two things in CUDA:
//  1. GPU timing -- record event before and after a kernel, measure elapsed
//  2. Stream synchronization -- one stream can wait for another's event
//
// In our simulator, an event wraps a Layer 5 Fence with a timestamp.
type CUDAEvent struct {
	fence     *cr.Fence
	timestamp int
	recorded  bool
}

// =========================================================================
// CUDARuntime -- the main simulator class
// =========================================================================

// CUDARuntime wraps Layer 5 with CUDA semantics.
//
// # Usage
//
//	cuda, _ := NewCUDARuntime()
//	dX, _ := cuda.Malloc(1024)
//	cuda.Memcpy(dX, nil, hostData, 1024, CUDAMemcpyHostToDevice)
//	cuda.LaunchKernel(kernel, grid, block, []*CUDADevicePtr{dX}, 0, nil)
//	cuda.DeviceSynchronize()
//	cuda.Free(dX)
type CUDARuntime struct {
	*BaseVendorSimulator
	deviceID int
	streams  []*CUDAStream
	events   []*CUDAEvent
}

// NewCUDARuntime creates a new CUDA runtime, selecting an NVIDIA GPU.
func NewCUDARuntime() (*CUDARuntime, error) {
	base, err := InitBase(nil, "nvidia")
	if err != nil {
		return nil, fmt.Errorf("failed to initialize CUDA runtime: %w", err)
	}
	return &CUDARuntime{
		BaseVendorSimulator: base,
	}, nil
}

// =================================================================
// Device management
// =================================================================

// SetDevice selects which GPU to use (cudaSetDevice).
//
// In multi-GPU systems, this switches the "current" device. In our
// simulator, we only model one device, so this validates the ID.
func (c *CUDARuntime) SetDevice(deviceID int) error {
	if deviceID < 0 || deviceID >= len(c.PhysicalDevices) {
		return fmt.Errorf(
			"invalid device ID %d (available: 0-%d)",
			deviceID, len(c.PhysicalDevices)-1,
		)
	}
	c.deviceID = deviceID
	return nil
}

// GetDevice returns the current device ID (cudaGetDevice).
func (c *CUDARuntime) GetDevice() int {
	return c.deviceID
}

// GetDeviceProperties queries device properties (cudaGetDeviceProperties).
func (c *CUDARuntime) GetDeviceProperties() CUDADeviceProperties {
	pd := c.PhysicalDevice
	memSize := 0
	for _, h := range pd.MemProperties().Heaps {
		memSize += h.Size
	}
	return CUDADeviceProperties{
		Name:               pd.Name(),
		TotalGlobalMem:     memSize,
		SharedMemPerBlock:  49152, // 48 KB
		MaxThreadsPerBlock: pd.Limits().MaxWorkgroupSize[0],
		MaxGridSize:        pd.Limits().MaxWorkgroupCount,
		WarpSize:           32,
		ComputeCapability:  [2]int{8, 0},
	}
}

// DeviceSynchronize waits for all GPU work to complete (cudaDeviceSynchronize).
//
// This is the bluntest synchronization tool -- it blocks the CPU until every
// kernel, every copy, every operation on every stream has finished.
func (c *CUDARuntime) DeviceSynchronize() {
	c.LogicalDevice.WaitIdle()
}

// DeviceReset resets the device (cudaDeviceReset).
//
// Destroys all allocations, streams, and state.
func (c *CUDARuntime) DeviceReset() {
	c.LogicalDevice.ResetDevice()
	c.streams = nil
	c.events = nil
}

// =================================================================
// Memory management
// =================================================================

// Malloc allocates device memory (cudaMalloc).
//
// Allocates GPU-accessible memory. We use HOST_VISIBLE | HOST_COHERENT for
// simulation convenience so we can actually read/write data from tests.
func (c *CUDARuntime) Malloc(size int) (*CUDADevicePtr, error) {
	buf, err := c.MemoryManager.Allocate(size, DefaultMemType(), DefaultUsage())
	if err != nil {
		return nil, fmt.Errorf("cudaMalloc failed: %w", err)
	}
	return &CUDADevicePtr{
		Buffer:        buf,
		DeviceAddress: buf.DeviceAddress,
		Size:          size,
	}, nil
}

// MallocManaged allocates unified/managed memory (cudaMallocManaged).
//
// Managed memory is accessible from both CPU and GPU. The CUDA runtime
// handles page migration automatically.
func (c *CUDARuntime) MallocManaged(size int) (*CUDADevicePtr, error) {
	buf, err := c.MemoryManager.Allocate(size, DefaultMemType(), DefaultUsage())
	if err != nil {
		return nil, fmt.Errorf("cudaMallocManaged failed: %w", err)
	}
	return &CUDADevicePtr{
		Buffer:        buf,
		DeviceAddress: buf.DeviceAddress,
		Size:          size,
	}, nil
}

// Free frees device memory (cudaFree).
func (c *CUDARuntime) Free(ptr *CUDADevicePtr) error {
	return c.MemoryManager.Free(ptr.Buffer)
}

// Memcpy copies memory between host and device (cudaMemcpy).
//
// # The Four Copy Directions
//
//	HostToDevice:   src is []byte (CPU), dstPtr is *CUDADevicePtr (GPU)
//	DeviceToHost:   srcPtr is *CUDADevicePtr (GPU), dstBuf is []byte (CPU)
//	DeviceToDevice: both srcPtr and dstPtr are *CUDADevicePtr
//	HostToHost:     both src and dstBuf are []byte
//
// Parameters:
//   - dstPtr: destination device pointer (for H2D, D2D)
//   - dstBuf: destination host buffer (for D2H, H2H)
//   - src: source data (host bytes for H2D, H2H) or nil
//   - srcPtr: source device pointer (for D2H, D2D) or nil
//   - size: number of bytes to copy
//   - kind: copy direction
func (c *CUDARuntime) Memcpy(
	dstPtr *CUDADevicePtr,
	dstBuf []byte,
	src []byte,
	srcPtr *CUDADevicePtr,
	size int,
	kind CUDAMemcpyKind,
) error {
	switch kind {
	case CUDAMemcpyHostToDevice:
		if dstPtr == nil {
			return fmt.Errorf("dst must be CUDADevicePtr for HostToDevice")
		}
		if src == nil {
			return fmt.Errorf("src must be []byte for HostToDevice")
		}
		mapped, err := c.MemoryManager.Map(dstPtr.Buffer)
		if err != nil {
			return err
		}
		copyLen := size
		if copyLen > len(src) {
			copyLen = len(src)
		}
		if err := mapped.Write(0, src[:copyLen]); err != nil {
			return err
		}
		return c.MemoryManager.Unmap(dstPtr.Buffer)

	case CUDAMemcpyDeviceToHost:
		if srcPtr == nil {
			return fmt.Errorf("src must be CUDADevicePtr for DeviceToHost")
		}
		if dstBuf == nil {
			return fmt.Errorf("dst must be []byte for DeviceToHost")
		}
		if err := c.MemoryManager.Invalidate(srcPtr.Buffer, 0, 0); err != nil {
			return err
		}
		mapped, err := c.MemoryManager.Map(srcPtr.Buffer)
		if err != nil {
			return err
		}
		data, err := mapped.Read(0, size)
		if err != nil {
			return err
		}
		if err := c.MemoryManager.Unmap(srcPtr.Buffer); err != nil {
			return err
		}
		copy(dstBuf[:size], data)
		return nil

	case CUDAMemcpyDeviceToDevice:
		if dstPtr == nil || srcPtr == nil {
			return fmt.Errorf("both src and dst must be CUDADevicePtr for DeviceToDevice")
		}
		_, err := c.CreateAndSubmitCB(func(cb *cr.CommandBuffer) error {
			return cb.CmdCopyBuffer(srcPtr.Buffer, dstPtr.Buffer, size, 0, 0)
		}, nil)
		return err

	case CUDAMemcpyHostToHost:
		if dstBuf == nil || src == nil {
			return fmt.Errorf("both src and dst must be []byte for HostToHost")
		}
		copy(dstBuf[:size], src[:size])
		return nil

	default:
		return fmt.Errorf("unknown memcpy kind: %d", kind)
	}
}

// Memset sets device memory to a value (cudaMemset).
//
// Fills the first `size` bytes of device memory with the byte value.
func (c *CUDARuntime) Memset(ptr *CUDADevicePtr, value int, size int) error {
	_, err := c.CreateAndSubmitCB(func(cb *cr.CommandBuffer) error {
		return cb.CmdFillBuffer(ptr.Buffer, value, 0, size)
	}, nil)
	return err
}

// =================================================================
// Kernel launch -- the heart of CUDA
// =================================================================

// LaunchKernel launches a CUDA kernel (the <<<grid, block>>> operator).
//
// # What Happens Internally
//
// This single call hides the entire Vulkan-style pipeline:
//
//  1. Create a ShaderModule from the kernel's code, with the
//     block dimensions as the local workgroup size.
//  2. Create a DescriptorSetLayout and PipelineLayout.
//  3. Create a Pipeline binding the shader to the layout.
//  4. Create a DescriptorSet and bind the argument buffers.
//  5. Create a CommandBuffer.
//  6. Record: bind_pipeline -> bind_descriptor_set -> dispatch.
//  7. Submit to the queue (default or specified stream).
//  8. Wait for completion.
func (c *CUDARuntime) LaunchKernel(
	kernel CUDAKernel,
	grid Dim3,
	block Dim3,
	args []*CUDADevicePtr,
	sharedMem int,
	stream *CUDAStream,
) error {
	device := c.LogicalDevice

	// Step 1: Create shader module with the kernel's code
	shader := device.CreateShaderModule(cr.ShaderModuleOptions{
		Code:      kernel.Code,
		LocalSize: [3]int{block.X, block.Y, block.Z},
	})

	// Step 2: Create descriptor set layout with one binding per argument
	bindings := make([]cr.DescriptorBinding, len(args))
	for i := range args {
		bindings[i] = cr.DescriptorBinding{Binding: i, Type: "storage", Count: 1}
	}
	dsLayout := device.CreateDescriptorSetLayout(bindings)
	plLayout := device.CreatePipelineLayout([]*cr.DescriptorSetLayout{dsLayout}, 0)

	// Step 3: Create the compute pipeline
	pipeline := device.CreateComputePipeline(shader, plLayout)

	// Step 4: Create and populate descriptor set
	ds := device.CreateDescriptorSet(dsLayout)
	for i, arg := range args {
		if err := ds.Write(i, arg.Buffer); err != nil {
			return fmt.Errorf("failed to bind arg %d: %w", i, err)
		}
	}

	// Step 5-8: Record and submit via helper
	var queue *cr.CommandQueue
	if stream != nil {
		queue = stream.queue
	}
	_, err := c.CreateAndSubmitCB(func(cb *cr.CommandBuffer) error {
		if err := cb.CmdBindPipeline(pipeline); err != nil {
			return err
		}
		if err := cb.CmdBindDescriptorSet(ds); err != nil {
			return err
		}
		return cb.CmdDispatch(grid.X, grid.Y, grid.Z)
	}, queue)
	return err
}

// =================================================================
// Streams
// =================================================================

// CreateStream creates a new CUDA stream (cudaStreamCreate).
//
// A stream is an independent execution queue. Operations enqueued to
// different streams can overlap.
func (c *CUDARuntime) CreateStream() *CUDAStream {
	stream := &CUDAStream{queue: c.ComputeQueue}
	c.streams = append(c.streams, stream)
	return stream
}

// DestroyStream destroys a CUDA stream (cudaStreamDestroy).
func (c *CUDARuntime) DestroyStream(stream *CUDAStream) error {
	for i, s := range c.streams {
		if s == stream {
			c.streams = append(c.streams[:i], c.streams[i+1:]...)
			return nil
		}
	}
	return fmt.Errorf("stream not found or already destroyed")
}

// StreamSynchronize waits for all operations in a stream (cudaStreamSynchronize).
func (c *CUDARuntime) StreamSynchronize(stream *CUDAStream) {
	if stream.pendingFence != nil {
		stream.pendingFence.Wait(nil)
	}
}

// =================================================================
// Events (for GPU timing)
// =================================================================

// CreateEvent creates a CUDA event (cudaEventCreate).
func (c *CUDARuntime) CreateEvent() *CUDAEvent {
	fence := c.LogicalDevice.CreateFence(false)
	event := &CUDAEvent{fence: fence}
	c.events = append(c.events, event)
	return event
}

// RecordEvent records an event in a stream (cudaEventRecord).
//
// Places a timestamp marker at the current position in the stream.
func (c *CUDARuntime) RecordEvent(event *CUDAEvent, stream *CUDAStream) {
	queue := c.ComputeQueue
	if stream != nil {
		queue = stream.queue
	}
	event.timestamp = queue.TotalCycles()
	event.fence.Signal()
	event.recorded = true
}

// SynchronizeEvent waits for an event to complete (cudaEventSynchronize).
func (c *CUDARuntime) SynchronizeEvent(event *CUDAEvent) error {
	if !event.recorded {
		return fmt.Errorf("event was never recorded")
	}
	event.fence.Wait(nil)
	return nil
}

// ElapsedTime measures elapsed GPU time between two events (cudaEventElapsedTime).
//
// Returns the time in milliseconds (simulated from cycle counts).
func (c *CUDARuntime) ElapsedTime(start, end *CUDAEvent) (float64, error) {
	if !start.recorded {
		return 0, fmt.Errorf("start event was never recorded")
	}
	if !end.recorded {
		return 0, fmt.Errorf("end event was never recorded")
	}
	cycles := end.timestamp - start.timestamp
	// Convert cycle difference to milliseconds (assume 1 GHz clock)
	return float64(cycles) / 1_000_000.0, nil
}

// DeviceCount returns the number of available CUDA devices.
func (c *CUDARuntime) DeviceCount() int {
	return len(c.PhysicalDevices)
}
