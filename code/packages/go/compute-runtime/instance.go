package computeruntime

// Instance -- device discovery, physical/logical device management.
//
// # The Entry Point
//
// The RuntimeInstance is how everything starts. It is the first object you
// create, and it gives you access to all available hardware:
//
//	instance := NewRuntimeInstance(nil)
//	devices := instance.EnumeratePhysicalDevices()
//	// -> [PhysicalDevice("NVIDIA H100"), PhysicalDevice("Apple M3 Max ANE"), ...]
//
// # Physical vs Logical Device
//
// A PhysicalDevice is a read-only description of hardware. You can query
// its name, type, memory, and capabilities, but you cannot use it directly.
//
// A LogicalDevice is a usable handle. It wraps a PhysicalDevice and provides:
//   - Command queues for submitting work
//   - Memory manager for allocating buffers
//   - Factory methods for pipelines, sync objects, etc.
//
// Why the separation?
//   - A system may have multiple GPUs. You query all of them, compare, and pick.
//   - Multiple logical devices can share one physical device.
//   - The physical device never changes. The logical device owns mutable state.

import (
	"math"

	devicesimulator "github.com/adhithyan15/coding-adventures/code/packages/go/device-simulator"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// PhysicalDevice -- read-only hardware description
// =========================================================================

// PhysicalDevice is a read-only description of a physical accelerator.
//
// You cannot execute anything on a PhysicalDevice. Create a LogicalDevice
// for that.
type PhysicalDevice struct {
	deviceID         int
	name             string
	deviceType       DeviceType
	vendor           string
	accelerator      devicesimulator.AcceleratorDevice
	memoryProperties MemoryProperties
	queueFamilies    []QueueFamily
	limits           DeviceLimits
}

// DeviceID returns the unique device identifier.
func (pd *PhysicalDevice) DeviceID() int { return pd.deviceID }

// Name returns the human-readable name.
func (pd *PhysicalDevice) Name() string { return pd.name }

// DeviceType returns GPU, TPU, or NPU.
func (pd *PhysicalDevice) DeviceType() DeviceType { return pd.deviceType }

// Vendor returns the vendor identifier.
func (pd *PhysicalDevice) Vendor() string { return pd.vendor }

// MemProperties returns the available memory types and heaps.
func (pd *PhysicalDevice) MemProperties() MemoryProperties { return pd.memoryProperties }

// QueueFamilies returns the available queue families.
func (pd *PhysicalDevice) QueueFamilies() []QueueFamily {
	result := make([]QueueFamily, len(pd.queueFamilies))
	copy(result, pd.queueFamilies)
	return result
}

// Limits returns the hardware limits.
func (pd *PhysicalDevice) Limits() DeviceLimits { return pd.limits }

// Accelerator returns the underlying Layer 6 device (internal use).
func (pd *PhysicalDevice) Accelerator() devicesimulator.AcceleratorDevice {
	return pd.accelerator
}

// SupportsFeature checks if a feature is supported.
//
// Currently supported features:
//   - "fp32": 32-bit float (always true)
//   - "fp16": 16-bit float (always true)
//   - "unified_memory": CPU/GPU shared memory
//   - "transfer_queue": dedicated DMA engine
func (pd *PhysicalDevice) SupportsFeature(feature string) bool {
	switch feature {
	case "fp32", "fp16":
		return true
	case "unified_memory":
		return pd.memoryProperties.IsUnified
	case "transfer_queue":
		for _, qf := range pd.queueFamilies {
			if qf.QueueType == QueueTypeTransfer {
				return true
			}
		}
		return false
	default:
		return false
	}
}

// =========================================================================
// LogicalDevice -- usable handle with queues and factories
// =========================================================================

// LogicalDevice is a usable device handle with command queues and resource factories.
type LogicalDevice struct {
	physical      *PhysicalDevice
	accelerator   devicesimulator.AcceleratorDevice
	queues        map[string][]*CommandQueue
	memoryManager *MemoryManager
	stats         *RuntimeStats
}

// PhysicalDevice returns the underlying physical device.
func (ld *LogicalDevice) PhysicalDevice() *PhysicalDevice { return ld.physical }

// Queues returns command queues by type name ("compute", "transfer").
func (ld *LogicalDevice) Queues() map[string][]*CommandQueue { return ld.queues }

// MemoryManager returns the memory allocation manager.
func (ld *LogicalDevice) MemoryManager() *MemoryManager { return ld.memoryManager }

// Stats returns runtime statistics.
func (ld *LogicalDevice) Stats() *RuntimeStats { return ld.stats }

// --- Factory methods ---

// CreateCommandBuffer creates a new command buffer.
func (ld *LogicalDevice) CreateCommandBuffer() *CommandBuffer {
	return NewCommandBuffer()
}

// CreateShaderModule creates a shader module from options.
func (ld *LogicalDevice) CreateShaderModule(opts ShaderModuleOptions) *ShaderModule {
	return NewShaderModule(opts)
}

// CreateDescriptorSetLayout creates a descriptor set layout.
func (ld *LogicalDevice) CreateDescriptorSetLayout(bindings []DescriptorBinding) *DescriptorSetLayout {
	return NewDescriptorSetLayout(bindings)
}

// CreatePipelineLayout creates a pipeline layout.
func (ld *LogicalDevice) CreatePipelineLayout(
	setLayouts []*DescriptorSetLayout,
	pushConstantSize int,
) *PipelineLayout {
	return NewPipelineLayout(setLayouts, pushConstantSize)
}

// CreateComputePipeline creates a compute pipeline.
func (ld *LogicalDevice) CreateComputePipeline(
	shader *ShaderModule,
	layout *PipelineLayout,
) *Pipeline {
	return NewPipeline(shader, layout)
}

// CreateDescriptorSet creates a descriptor set from a layout.
func (ld *LogicalDevice) CreateDescriptorSet(layout *DescriptorSetLayout) *DescriptorSet {
	return NewDescriptorSet(layout)
}

// CreateFence creates a fence for CPU<->GPU synchronization.
func (ld *LogicalDevice) CreateFence(signaled bool) *Fence {
	return NewFence(signaled)
}

// CreateSemaphore creates a semaphore for GPU queue<->queue synchronization.
func (ld *LogicalDevice) CreateSemaphore() *Semaphore {
	return NewSemaphore()
}

// CreateEvent creates an event for fine-grained GPU-side signaling.
func (ld *LogicalDevice) CreateEvent() *Event {
	return NewEvent()
}

// WaitIdle blocks until all queues finish all pending work.
func (ld *LogicalDevice) WaitIdle() {
	for _, queueList := range ld.queues {
		for _, queue := range queueList {
			queue.WaitIdle()
		}
	}
}

// ResetDevice resets all device state.
func (ld *LogicalDevice) ResetDevice() {
	ld.accelerator.Reset()
}

// =========================================================================
// RuntimeInstance -- the entry point
// =========================================================================

// makePhysicalDevice creates a PhysicalDevice from an AcceleratorDevice.
func makePhysicalDevice(
	deviceID int,
	accelerator devicesimulator.AcceleratorDevice,
	deviceType DeviceType,
	vendor string,
) *PhysicalDevice {
	config := accelerator.Config()
	isUnified := config.UnifiedMemory

	var heaps []MemoryHeap
	if isUnified {
		heaps = []MemoryHeap{{
			Size:  config.GlobalMemorySize,
			Flags: MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent,
		}}
	} else {
		stagingSize := config.GlobalMemorySize / 4
		maxStaging := 256 * 1024 * 1024
		if stagingSize > maxStaging {
			stagingSize = maxStaging
		}
		heaps = []MemoryHeap{
			{
				Size:  config.GlobalMemorySize,
				Flags: MemoryTypeDeviceLocal,
			},
			{
				Size:  stagingSize,
				Flags: MemoryTypeHostVisible | MemoryTypeHostCoherent,
			},
		}
	}

	memProperties := MemoryProperties{Heaps: heaps, IsUnified: isUnified}

	queueFamilies := []QueueFamily{
		{QueueType: QueueTypeCompute, Count: 4},
	}
	if !isUnified {
		queueFamilies = append(queueFamilies, QueueFamily{
			QueueType: QueueTypeTransfer,
			Count:     2,
		})
	}

	return &PhysicalDevice{
		deviceID:         deviceID,
		name:             accelerator.Name(),
		deviceType:       deviceType,
		vendor:           vendor,
		accelerator:      accelerator,
		memoryProperties: memProperties,
		queueFamilies:    queueFamilies,
		limits:           DefaultDeviceLimits(),
	}
}

// DeviceEntry describes a device to register with the runtime instance.
type DeviceEntry struct {
	Accelerator devicesimulator.AcceleratorDevice
	Type        DeviceType
	Vendor      string
}

// RuntimeInstance is the runtime entry point -- discovers devices and creates handles.
type RuntimeInstance struct {
	version         string
	physicalDevices []*PhysicalDevice
}

// NewRuntimeInstance creates a runtime instance.
//
// If devices is nil, creates default test devices (small configs).
func NewRuntimeInstance(devices []DeviceEntry) *RuntimeInstance {
	inst := &RuntimeInstance{version: "0.1.0"}

	if devices != nil {
		for i, entry := range devices {
			inst.physicalDevices = append(inst.physicalDevices,
				makePhysicalDevice(i, entry.Accelerator, entry.Type, entry.Vendor),
			)
		}
	} else {
		inst.physicalDevices = createDefaultDevices()
	}

	return inst
}

// Version returns the runtime version string.
func (ri *RuntimeInstance) Version() string { return ri.version }

// EnumeratePhysicalDevices returns all available physical devices.
func (ri *RuntimeInstance) EnumeratePhysicalDevices() []*PhysicalDevice {
	result := make([]*PhysicalDevice, len(ri.physicalDevices))
	copy(result, ri.physicalDevices)
	return result
}

// QueueRequest specifies what queues to create on a logical device.
type QueueRequest struct {
	Type  string // "compute", "transfer", or "compute_transfer"
	Count int
}

// CreateLogicalDevice creates a logical device from a physical device.
func (ri *RuntimeInstance) CreateLogicalDevice(
	physicalDevice *PhysicalDevice,
	queueRequests []QueueRequest,
) *LogicalDevice {
	if len(queueRequests) == 0 {
		queueRequests = []QueueRequest{{Type: "compute", Count: 1}}
	}

	stats := &RuntimeStats{}
	accelerator := physicalDevice.accelerator

	memoryManager := NewMemoryManager(accelerator, physicalDevice.memoryProperties, stats)

	queues := make(map[string][]*CommandQueue)
	for _, req := range queueRequests {
		count := req.Count
		if count <= 0 {
			count = 1
		}
		qt := ParseQueueType(req.Type)
		queueList := make([]*CommandQueue, count)
		for i := 0; i < count; i++ {
			queueList[i] = NewCommandQueue(qt, i, accelerator, memoryManager, stats)
		}
		queues[req.Type] = queueList
	}

	return &LogicalDevice{
		physical:      physicalDevice,
		accelerator:   accelerator,
		queues:        queues,
		memoryManager: memoryManager,
		stats:         stats,
	}
}

// createDefaultDevices creates small default devices for testing.
func createDefaultDevices() []*PhysicalDevice {
	type defaultDev struct {
		create func() devicesimulator.AcceleratorDevice
		dtype  DeviceType
		vendor string
	}

	defaults := []defaultDev{
		{
			create: func() devicesimulator.AcceleratorDevice {
				return devicesimulator.NewNvidiaGPU(nil, 2)
			},
			dtype:  DeviceTypeGPU,
			vendor: "nvidia",
		},
		{
			create: func() devicesimulator.AcceleratorDevice {
				return devicesimulator.NewAmdGPU(nil, 2)
			},
			dtype:  DeviceTypeGPU,
			vendor: "amd",
		},
		{
			create: func() devicesimulator.AcceleratorDevice {
				return devicesimulator.NewGoogleTPU(nil, 2)
			},
			dtype:  DeviceTypeTPU,
			vendor: "google",
		},
		{
			create: func() devicesimulator.AcceleratorDevice {
				return devicesimulator.NewIntelGPU(nil, 2)
			},
			dtype:  DeviceTypeGPU,
			vendor: "intel",
		},
		{
			create: func() devicesimulator.AcceleratorDevice {
				return devicesimulator.NewAppleANE(nil, 2)
			},
			dtype:  DeviceTypeNPU,
			vendor: "apple",
		},
	}

	result := make([]*PhysicalDevice, len(defaults))
	for i, d := range defaults {
		result[i] = makePhysicalDevice(i, d.create(), d.dtype, d.vendor)
	}
	return result
}

// Ensure gpucore import is used (for ShaderModuleOptions).
var _ = gpucore.Instruction{}

// Ensure math is used indirectly via downstream code or suppress warning.
var _ = math.MaxInt
