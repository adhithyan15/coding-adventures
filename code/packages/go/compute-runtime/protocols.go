// Package computeruntime implements Layer 5 of the accelerator computing stack --
// the software layer between user-facing APIs (CUDA, OpenCL, Metal, Vulkan) and
// the hardware device simulators (Layer 6).
//
// # What is a Compute Runtime?
//
// A compute runtime is the GPU driver's internal machinery:
//
//	User code:     y = alpha * x + y
//	     |
//	API layer:     cudaMalloc / vkAllocateMemory / MTLBuffer  (Layer 4, future)
//	     |
//	Runtime:       CommandBuffer, Queue, Fence, Memory types   (THIS LAYER)
//	     |
//	Hardware:      NvidiaGPU.LaunchKernel, .Step(), .Run()     (Layer 6)
//
// The runtime manages:
//   - Device discovery and selection (Instance -> PhysicalDevice -> LogicalDevice)
//   - Memory allocation with types (DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT)
//   - Command recording and submission (CommandBuffer -> CommandQueue)
//   - Synchronization (Fence, Semaphore, Event, PipelineBarrier)
//   - Pipeline and descriptor management (ShaderModule -> Pipeline -> DescriptorSet)
//
// # Why Vulkan-Inspired?
//
// Vulkan is the most explicit GPU API -- it exposes every moving part that
// CUDA, OpenCL, and Metal hide behind convenience wrappers. If we model at
// Vulkan's level, building the other APIs on top becomes straightforward:
//
//	Vulkan:   "Here's a command buffer with barriers and descriptor sets"
//	CUDA:     "Here's a kernel launch" (implicitly creates CB, barriers, etc.)
//	Metal:    "Here's a command encoder" (like CB but with Apple conventions)
//	OpenCL:   "Here's a kernel with args" (like CUDA but cross-platform)
//
// # Design Principle: Enums as Documentation
//
// Every enum-like const block in this file represents a real GPU concept. The
// values are not arbitrary -- they map to actual hardware states, memory types,
// and pipeline stages that exist in every GPU driver.
package computeruntime

import "fmt"

// =========================================================================
// DeviceType -- what kind of accelerator this is
// =========================================================================

// DeviceType identifies the class of accelerator hardware.
//
// # The Three Families
//
// GPU: General-purpose, thread-parallel. Thousands of small cores
// running the same program on different data (SIMT/SIMD).
// NVIDIA, AMD, Intel, Apple (GPU portion).
//
// TPU: Dataflow, matrix-specialized. One large matrix unit (MXU)
// that processes tiles of matrices in a pipeline.
// Google TPU.
//
// NPU: Neural processing unit. Fixed-function for inference,
// with compiler-generated execution schedules.
// Apple ANE, Qualcomm Hexagon, Intel NPU.
type DeviceType int

const (
	DeviceTypeGPU DeviceType = iota
	DeviceTypeTPU
	DeviceTypeNPU
)

// String returns a human-readable name for the device type.
func (d DeviceType) String() string {
	result, _ := StartNew[string]("compute-runtime.DeviceType.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			switch d {
			case DeviceTypeGPU:
				return rf.Generate(true, false, "gpu")
			case DeviceTypeTPU:
				return rf.Generate(true, false, "tpu")
			case DeviceTypeNPU:
				return rf.Generate(true, false, "npu")
			default:
				return rf.Generate(true, false, "unknown")
			}
		}).GetResult()
	return result
}

// =========================================================================
// QueueType -- what kind of work a queue can accept
// =========================================================================

// QueueType identifies the kind of work a command queue can accept.
//
// # Why Multiple Queue Types?
//
// Real GPUs have separate hardware engines for compute and data transfer.
// While the compute engine runs a kernel, the DMA engine can copy data
// for the next kernel in parallel. This overlap hides PCIe latency.
//
//	Compute Queue:   [Kernel A]----------[Kernel B]----------
//	Transfer Queue:  -----[Upload B data]-----[Upload C data]---
//
// Without separate queues, you would have to wait:
//
//	Single Queue:    [Upload]--[Kernel A]--[Upload]--[Kernel B]--
//	                  ^^^^^^^^               ^^^^^^^^
//	                  GPU idle               GPU idle
type QueueType int

const (
	// QueueTypeCompute can run kernels (dispatch commands).
	QueueTypeCompute QueueType = iota
	// QueueTypeTransfer can copy data (DMA engine).
	QueueTypeTransfer
	// QueueTypeComputeTransfer can do both (most common on simple devices).
	QueueTypeComputeTransfer
)

// String returns a human-readable name for the queue type.
func (q QueueType) String() string {
	result, _ := StartNew[string]("compute-runtime.QueueType.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			switch q {
			case QueueTypeCompute:
				return rf.Generate(true, false, "compute")
			case QueueTypeTransfer:
				return rf.Generate(true, false, "transfer")
			case QueueTypeComputeTransfer:
				return rf.Generate(true, false, "compute_transfer")
			default:
				return rf.Generate(true, false, "unknown")
			}
		}).GetResult()
	return result
}

// ParseQueueType converts a string to a QueueType.
// Returns QueueTypeComputeTransfer if the string is unrecognized.
func ParseQueueType(s string) QueueType {
	result, _ := StartNew[QueueType]("compute-runtime.ParseQueueType", 0,
		func(op *Operation[QueueType], rf *ResultFactory[QueueType]) *OperationResult[QueueType] {
			op.AddProperty("input", s)
			switch s {
			case "compute":
				return rf.Generate(true, false, QueueTypeCompute)
			case "transfer":
				return rf.Generate(true, false, QueueTypeTransfer)
			case "compute_transfer":
				return rf.Generate(true, false, QueueTypeComputeTransfer)
			default:
				return rf.Generate(true, false, QueueTypeComputeTransfer)
			}
		}).GetResult()
	return result
}

// =========================================================================
// MemoryType -- properties of a memory allocation (flags, combinable)
// =========================================================================

// MemoryType describes properties of a memory allocation. These are bit flags
// that can be combined with | (bitwise OR) to describe memory with multiple
// properties.
//
// # Memory Types Explained
//
// DEVICE_LOCAL:
//
//	Fast GPU memory (VRAM / HBM). The GPU can access this at full
//	bandwidth (1-3 TB/s). The CPU CANNOT directly read/write this
//	unless HOST_VISIBLE is also set.
//
// HOST_VISIBLE:
//
//	The CPU can map this memory and read/write it. On discrete GPUs,
//	this is typically a small pool of system RAM accessible via PCIe.
//	On unified memory, all memory is HOST_VISIBLE.
//
// HOST_COHERENT:
//
//	CPU writes are immediately visible to the GPU without explicit
//	flush. More convenient but may be slower.
//
// HOST_CACHED:
//
//	CPU reads are cached (fast read-back). Without this, every CPU
//	read goes over PCIe -- very slow.
//
// # Common Combinations
//
//	DEVICE_LOCAL                              -> GPU-only, fastest
//	HOST_VISIBLE | HOST_COHERENT              -> staging buffer for uploads
//	HOST_VISIBLE | HOST_CACHED                -> read-back buffer for downloads
//	DEVICE_LOCAL | HOST_VISIBLE               -> unified memory (Apple, resizable BAR)
//	DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT -> zero-copy unified
type MemoryType uint32

const (
	MemoryTypeDeviceLocal  MemoryType = 1 << iota // GPU-only, fastest
	MemoryTypeHostVisible                          // CPU can map and access
	MemoryTypeHostCoherent                         // CPU writes immediately visible to GPU
	MemoryTypeHostCached                           // CPU reads are cached
)

// Has checks whether m contains all flags in other.
func (m MemoryType) Has(other MemoryType) bool {
	result, _ := StartNew[bool]("compute-runtime.MemoryType.Has", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, m&other == other)
		}).GetResult()
	return result
}

// String returns a human-readable representation of memory type flags.
func (m MemoryType) String() string {
	result, _ := StartNew[string]("compute-runtime.MemoryType.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			if m == 0 {
				return rf.Generate(true, false, "NONE")
			}
			names := []string{}
			if m&MemoryTypeDeviceLocal != 0 {
				names = append(names, "DEVICE_LOCAL")
			}
			if m&MemoryTypeHostVisible != 0 {
				names = append(names, "HOST_VISIBLE")
			}
			if m&MemoryTypeHostCoherent != 0 {
				names = append(names, "HOST_COHERENT")
			}
			if m&MemoryTypeHostCached != 0 {
				names = append(names, "HOST_CACHED")
			}
			res := ""
			for i, n := range names {
				if i > 0 {
					res += " | "
				}
				res += n
			}
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// =========================================================================
// BufferUsage -- how a buffer will be used (flags, combinable)
// =========================================================================

// BufferUsage describes how a buffer will be used. These are bit flags
// that can be combined with | (bitwise OR).
//
// # Why Declare Usage?
//
// Telling the GPU how a buffer will be used enables optimizations:
//   - STORAGE buffers may be placed in faster memory regions
//   - TRANSFER_SRC buffers can be DMA-aligned for faster copies
//   - UNIFORM buffers may be cached in special constant caches
//
// You must declare all intended usages at allocation time. Using a
// buffer in a way not declared is a validation error.
type BufferUsage uint32

const (
	// BufferUsageStorage -- shader/kernel can read and write (SSBO in Vulkan, CUDA global mem).
	BufferUsageStorage BufferUsage = 1 << iota
	// BufferUsageUniform -- shader/kernel can only read. Small, fast (UBO in Vulkan).
	BufferUsageUniform
	// BufferUsageTransferSrc -- can be the source of a copy command.
	BufferUsageTransferSrc
	// BufferUsageTransferDst -- can be the destination of a copy command.
	BufferUsageTransferDst
	// BufferUsageIndirect -- contains indirect dispatch parameters.
	BufferUsageIndirect
)

// Has checks whether u contains all flags in other.
func (u BufferUsage) Has(other BufferUsage) bool {
	result, _ := StartNew[bool]("compute-runtime.BufferUsage.Has", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, u&other == other)
		}).GetResult()
	return result
}

// =========================================================================
// PipelineStage -- where in the GPU pipeline an operation happens
// =========================================================================

// PipelineStage identifies where in the GPU pipeline an operation happens.
//
// Commands flow through stages in order:
//
//	TOP_OF_PIPE -> TRANSFER -> COMPUTE -> BOTTOM_OF_PIPE
//	      |                                    |
//	      |          HOST (CPU access)          |
//	      +------------------------------------+
//
// When you create a barrier, you specify:
//   - src_stage: "wait until this stage finishes"
//   - dst_stage: "before this stage starts"
type PipelineStage int

const (
	// PipelineStageTopOfPipe is a virtual stage at the very beginning.
	PipelineStageTopOfPipe PipelineStage = iota
	// PipelineStageCompute is where compute shader / kernel execution happens.
	PipelineStageCompute
	// PipelineStageTransfer is where copy / fill / update buffer operations happen.
	PipelineStageTransfer
	// PipelineStageHost is where CPU access (map, read, write) happens.
	PipelineStageHost
	// PipelineStageBottomOfPipe is a virtual stage at the very end.
	PipelineStageBottomOfPipe
)

// String returns a human-readable name for the pipeline stage.
func (p PipelineStage) String() string {
	result, _ := StartNew[string]("compute-runtime.PipelineStage.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			switch p {
			case PipelineStageTopOfPipe:
				return rf.Generate(true, false, "top_of_pipe")
			case PipelineStageCompute:
				return rf.Generate(true, false, "compute")
			case PipelineStageTransfer:
				return rf.Generate(true, false, "transfer")
			case PipelineStageHost:
				return rf.Generate(true, false, "host")
			case PipelineStageBottomOfPipe:
				return rf.Generate(true, false, "bottom_of_pipe")
			default:
				return rf.Generate(true, false, "unknown")
			}
		}).GetResult()
	return result
}

// =========================================================================
// AccessFlags -- what kind of memory access an operation performs
// =========================================================================

// AccessFlags describes what kind of memory access an operation performs.
//
// GPUs have caches. When a kernel writes to a buffer, the data may sit
// in L2 cache, not yet visible to a subsequent kernel reading the same
// buffer. A memory barrier with the right access flags ensures caches
// are flushed (for writes) or invalidated (for reads).
type AccessFlags uint32

const (
	AccessFlagsNone         AccessFlags = 0
	AccessFlagsShaderRead   AccessFlags = 1 << iota // Compute kernel reads from a buffer.
	AccessFlagsShaderWrite                           // Compute kernel writes to a buffer.
	AccessFlagsTransferRead                          // Copy command reads from source buffer.
	AccessFlagsTransferWrite                         // Copy command writes to destination buffer.
	AccessFlagsHostRead                              // CPU reads mapped buffer.
	AccessFlagsHostWrite                             // CPU writes mapped buffer.
)

// =========================================================================
// CommandBufferState -- lifecycle state of a command buffer
// =========================================================================

// CommandBufferState represents the lifecycle state of a command buffer.
//
// # State Machine
//
//	INITIAL --begin()--> RECORDING --end()--> RECORDED
//	    ^                                        |
//	    |                                    submit()
//	    |                                        |
//	    +-------- reset() <-- COMPLETE <-- PENDING
//	                              |
//	                              +-- GPU finished
type CommandBufferState int

const (
	CommandBufferStateInitial   CommandBufferState = iota
	CommandBufferStateRecording
	CommandBufferStateRecorded
	CommandBufferStatePending
	CommandBufferStateComplete
)

// String returns a human-readable name for the command buffer state.
func (s CommandBufferState) String() string {
	result, _ := StartNew[string]("compute-runtime.CommandBufferState.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			switch s {
			case CommandBufferStateInitial:
				return rf.Generate(true, false, "initial")
			case CommandBufferStateRecording:
				return rf.Generate(true, false, "recording")
			case CommandBufferStateRecorded:
				return rf.Generate(true, false, "recorded")
			case CommandBufferStatePending:
				return rf.Generate(true, false, "pending")
			case CommandBufferStateComplete:
				return rf.Generate(true, false, "complete")
			default:
				return rf.Generate(true, false, "unknown")
			}
		}).GetResult()
	return result
}

// =========================================================================
// RuntimeEventType -- types of events the runtime can produce
// =========================================================================

// RuntimeEventType identifies the kind of runtime event logged in traces.
type RuntimeEventType int

const (
	RuntimeEventSubmit          RuntimeEventType = iota
	RuntimeEventBeginExecution
	RuntimeEventEndExecution
	RuntimeEventFenceSignal
	RuntimeEventFenceWait
	RuntimeEventSemaphoreSignal
	RuntimeEventSemaphoreWait
	RuntimeEventBarrier
	RuntimeEventMemoryAlloc
	RuntimeEventMemoryFree
	RuntimeEventMemoryMap
	RuntimeEventMemoryTransfer
)

// String returns a human-readable name for the runtime event type.
func (e RuntimeEventType) String() string {
	result, _ := StartNew[string]("compute-runtime.RuntimeEventType.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			switch e {
			case RuntimeEventSubmit:
				return rf.Generate(true, false, "SUBMIT")
			case RuntimeEventBeginExecution:
				return rf.Generate(true, false, "BEGIN_EXECUTION")
			case RuntimeEventEndExecution:
				return rf.Generate(true, false, "END_EXECUTION")
			case RuntimeEventFenceSignal:
				return rf.Generate(true, false, "FENCE_SIGNAL")
			case RuntimeEventFenceWait:
				return rf.Generate(true, false, "FENCE_WAIT")
			case RuntimeEventSemaphoreSignal:
				return rf.Generate(true, false, "SEMAPHORE_SIGNAL")
			case RuntimeEventSemaphoreWait:
				return rf.Generate(true, false, "SEMAPHORE_WAIT")
			case RuntimeEventBarrier:
				return rf.Generate(true, false, "BARRIER")
			case RuntimeEventMemoryAlloc:
				return rf.Generate(true, false, "MEMORY_ALLOC")
			case RuntimeEventMemoryFree:
				return rf.Generate(true, false, "MEMORY_FREE")
			case RuntimeEventMemoryMap:
				return rf.Generate(true, false, "MEMORY_MAP")
			case RuntimeEventMemoryTransfer:
				return rf.Generate(true, false, "MEMORY_TRANSFER")
			default:
				return rf.Generate(true, false, "UNKNOWN")
			}
		}).GetResult()
	return result
}

// =========================================================================
// QueueFamily -- describes what a queue can do
// =========================================================================

// QueueFamily describes a family of queues with the same capabilities.
//
// A GPU might have:
//   - 1 family of 16 compute queues
//   - 1 family of 2 transfer-only queues
//
// You request queues from families when creating a logical device.
type QueueFamily struct {
	// QueueType is what kind of work this family can do.
	QueueType QueueType
	// Count is how many queues are available in this family.
	Count int
}

// =========================================================================
// DeviceLimits -- hardware limits of a device
// =========================================================================

// DeviceLimits holds hardware limits of a device.
//
// These constrain what you can do -- exceeding them is a validation error.
type DeviceLimits struct {
	MaxWorkgroupSize     [3]int // Maximum threads in one workgroup (e.g., 1024).
	MaxWorkgroupCount    [3]int // Maximum workgroups per dispatch dimension.
	MaxBufferSize        int    // Largest single buffer allocation.
	MaxPushConstantSize  int    // Max bytes for push constants.
	MaxDescriptorSets    int    // Max descriptor sets bound simultaneously.
	MaxBindingsPerSet    int    // Max bindings in one descriptor set.
	MaxComputeQueues     int    // Max compute queues.
	MaxTransferQueues    int    // Max transfer-only queues.
}

// DefaultDeviceLimits returns DeviceLimits with sensible defaults.
func DefaultDeviceLimits() DeviceLimits {
	result, _ := StartNew[DeviceLimits]("compute-runtime.DefaultDeviceLimits", DeviceLimits{},
		func(op *Operation[DeviceLimits], rf *ResultFactory[DeviceLimits]) *OperationResult[DeviceLimits] {
			return rf.Generate(true, false, DeviceLimits{
				MaxWorkgroupSize:    [3]int{1024, 1024, 64},
				MaxWorkgroupCount:   [3]int{65535, 65535, 65535},
				MaxBufferSize:       2 * 1024 * 1024 * 1024, // 2 GB
				MaxPushConstantSize: 128,
				MaxDescriptorSets:   4,
				MaxBindingsPerSet:   16,
				MaxComputeQueues:    16,
				MaxTransferQueues:   2,
			})
		}).GetResult()
	return result
}

// =========================================================================
// MemoryHeap -- a physical pool of memory
// =========================================================================

// MemoryHeap represents a physical pool of memory on the device.
//
// Discrete GPUs typically have two heaps:
//   - VRAM (large, fast, DEVICE_LOCAL)
//   - System RAM (smaller GPU-visible portion, HOST_VISIBLE)
//
// Unified memory devices have one heap with all flags.
type MemoryHeap struct {
	Size  int        // Total size in bytes.
	Flags MemoryType // What memory types this heap supports.
}

// =========================================================================
// MemoryProperties -- all memory heaps and types available on a device
// =========================================================================

// MemoryProperties describes all memory heaps and types available on a device.
type MemoryProperties struct {
	Heaps     []MemoryHeap // Physical memory pools.
	IsUnified bool         // True if CPU and GPU share memory (Apple).
}

// =========================================================================
// DescriptorBinding -- one binding slot in a descriptor set layout
// =========================================================================

// DescriptorBinding describes one binding slot in a descriptor set layout.
//
// A descriptor is how you tell a kernel "buffer X is at binding slot 0."
// The kernel code references bindings by number, and the descriptor set
// maps those numbers to actual GPU memory addresses.
type DescriptorBinding struct {
	Binding int    // Slot number (0, 1, 2, ...).
	Type    string // "storage" (read/write) or "uniform" (read-only).
	Count   int    // Number of buffers at this binding (usually 1).
}

// DefaultDescriptorBinding returns a DescriptorBinding with sensible defaults.
func DefaultDescriptorBinding(binding int) DescriptorBinding {
	result, _ := StartNew[DescriptorBinding]("compute-runtime.DefaultDescriptorBinding", DescriptorBinding{},
		func(op *Operation[DescriptorBinding], rf *ResultFactory[DescriptorBinding]) *OperationResult[DescriptorBinding] {
			op.AddProperty("binding", binding)
			return rf.Generate(true, false, DescriptorBinding{
				Binding: binding,
				Type:    "storage",
				Count:   1,
			})
		}).GetResult()
	return result
}

// =========================================================================
// RecordedCommand -- stored inside command buffers
// =========================================================================

// RecordedCommand is a single command recorded into a command buffer.
//
// This is a simple tagged union: the Command field identifies the type,
// and Args holds command-specific data.
type RecordedCommand struct {
	Command string                 // Command type (e.g., "dispatch", "copy_buffer").
	Args    map[string]interface{} // Command-specific arguments.
}

// =========================================================================
// MemoryBarrier -- a memory ordering constraint
// =========================================================================

// MemoryBarrier is a memory ordering constraint.
//
// GPUs have caches. When kernel A writes to a buffer and kernel B reads
// from it, the writes may still be in L2 cache -- invisible to kernel B.
// A memory barrier flushes writes and invalidates read caches.
type MemoryBarrier struct {
	SrcAccess AccessFlags // What the previous operation did.
	DstAccess AccessFlags // What the next operation will do.
}

// =========================================================================
// BufferBarrier -- a barrier targeting a specific buffer
// =========================================================================

// BufferBarrier is a barrier targeting a specific buffer.
//
// Like MemoryBarrier, but scoped to one buffer. More efficient because
// the GPU only needs to flush/invalidate caches for that buffer.
type BufferBarrier struct {
	BufferID  int         // Which buffer this barrier applies to.
	SrcAccess AccessFlags // Previous access type.
	DstAccess AccessFlags // Next access type.
	Offset    int         // Start of affected region within the buffer.
	Size      int         // Size of affected region (0 = whole buffer).
}

// =========================================================================
// PipelineBarrier -- a full pipeline barrier
// =========================================================================

// PipelineBarrier is a full pipeline barrier with stage and memory constraints.
//
// # Anatomy of a Barrier
//
//	cmd_dispatch(kernel_A)
//	cmd_pipeline_barrier(PipelineBarrier{
//	    SrcStage: PipelineStageCompute,     // "wait for compute to finish"
//	    DstStage: PipelineStageCompute,     // "before starting next compute"
//	    MemoryBarriers: []MemoryBarrier{    // "and flush/invalidate memory"
//	        {SrcAccess: AccessFlagsShaderWrite, DstAccess: AccessFlagsShaderRead},
//	    },
//	})
//	cmd_dispatch(kernel_B)
type PipelineBarrierDesc struct {
	SrcStage        PipelineStage   // Wait until this stage completes.
	DstStage        PipelineStage   // Before this stage begins.
	MemoryBarriers  []MemoryBarrier // Global memory ordering.
	BufferBarriers  []BufferBarrier // Per-buffer memory ordering.
}

// DefaultPipelineBarrier returns a PipelineBarrierDesc with default stages.
func DefaultPipelineBarrier() PipelineBarrierDesc {
	result, _ := StartNew[PipelineBarrierDesc]("compute-runtime.DefaultPipelineBarrier", PipelineBarrierDesc{},
		func(op *Operation[PipelineBarrierDesc], rf *ResultFactory[PipelineBarrierDesc]) *OperationResult[PipelineBarrierDesc] {
			return rf.Generate(true, false, PipelineBarrierDesc{
				SrcStage: PipelineStageTopOfPipe,
				DstStage: PipelineStageBottomOfPipe,
			})
		}).GetResult()
	return result
}

// =========================================================================
// RuntimeTrace -- submission-level observability
// =========================================================================

// RuntimeTrace records one runtime-level event.
//
// # Device Traces vs Runtime Traces
//
// Device traces (Layer 6) are per-cycle: "SM 7 dispatched warp 42."
// Runtime traces are per-submission: "CB#1 submitted to compute queue."
//
// Together they give you the full picture -- what the software did (runtime)
// and what the hardware did in response (device).
type RuntimeTrace struct {
	TimestampCycles int              // When this event occurred (cumulative device cycles).
	EventType       RuntimeEventType // What happened.
	Description     string           // Human-readable summary.
	QueueType       *QueueType       // Which queue was involved (nil if N/A).
	CommandBufferID *int             // Which command buffer (nil if N/A).
	FenceID         *int             // Which fence (nil if N/A).
	SemaphoreID     *int             // Which semaphore (nil if N/A).
	DeviceTraces    []interface{}    // Hardware traces generated by this event.
}

// Format returns a human-readable summary of the trace.
//
// Example:
//
//	[T=150 cycles] SUBMIT -- CB#1 to compute queue
func (t RuntimeTrace) Format() string {
	result, _ := StartNew[string]("compute-runtime.RuntimeTrace.Format", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			res := fmt.Sprintf("[T=%d cycles] %s", t.TimestampCycles, t.EventType.String())
			if t.Description != "" {
				res += " -- " + t.Description
			}
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// =========================================================================
// RuntimeStats -- aggregate metrics
// =========================================================================

// RuntimeStats holds aggregate statistics for the entire runtime session.
//
// # Key Metrics
//
//	gpu_utilization = total_device_cycles / (total_device_cycles + total_idle_cycles)
//
// A well-utilized GPU has utilization close to 1.0 -- it is always busy.
// Low utilization means the CPU is bottlenecking the GPU or synchronization
// overhead is too high.
type RuntimeStats struct {
	// Submissions
	TotalSubmissions    int
	TotalCommandBuffers int
	TotalDispatches     int
	TotalTransfers      int
	TotalBarriers       int

	// Synchronization
	TotalFenceWaits      int
	TotalSemaphoreSignals int
	TotalFenceWaitCycles int

	// Memory
	TotalAllocatedBytes int
	PeakAllocatedBytes  int
	TotalAllocations    int
	TotalFrees          int
	TotalMaps           int

	// Timing
	TotalDeviceCycles int
	TotalIdleCycles   int
	GPUUtilization    float64

	// Traces
	Traces []RuntimeTrace
}

// UpdateUtilization recalculates GPU utilization from current counts.
func (s *RuntimeStats) UpdateUtilization() {
	_, _ = StartNew[struct{}]("compute-runtime.RuntimeStats.UpdateUtilization", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			total := s.TotalDeviceCycles + s.TotalIdleCycles
			if total > 0 {
				s.GPUUtilization = float64(s.TotalDeviceCycles) / float64(total)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
