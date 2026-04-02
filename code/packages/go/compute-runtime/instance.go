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
func (pd *PhysicalDevice) DeviceID() int {
	result, _ := StartNew[int]("compute-runtime.PhysicalDevice.DeviceID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, pd.deviceID)
		}).GetResult()
	return result
}

// Name returns the human-readable name.
func (pd *PhysicalDevice) Name() string {
	result, _ := StartNew[string]("compute-runtime.PhysicalDevice.Name", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, pd.name)
		}).GetResult()
	return result
}

// DeviceType returns GPU, TPU, or NPU.
func (pd *PhysicalDevice) DeviceType() DeviceType {
	result, _ := StartNew[DeviceType]("compute-runtime.PhysicalDevice.DeviceType", 0,
		func(op *Operation[DeviceType], rf *ResultFactory[DeviceType]) *OperationResult[DeviceType] {
			return rf.Generate(true, false, pd.deviceType)
		}).GetResult()
	return result
}

// Vendor returns the vendor identifier.
func (pd *PhysicalDevice) Vendor() string {
	result, _ := StartNew[string]("compute-runtime.PhysicalDevice.Vendor", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, pd.vendor)
		}).GetResult()
	return result
}

// MemProperties returns the available memory types and heaps.
func (pd *PhysicalDevice) MemProperties() MemoryProperties {
	result, _ := StartNew[MemoryProperties]("compute-runtime.PhysicalDevice.MemProperties", MemoryProperties{},
		func(op *Operation[MemoryProperties], rf *ResultFactory[MemoryProperties]) *OperationResult[MemoryProperties] {
			return rf.Generate(true, false, pd.memoryProperties)
		}).GetResult()
	return result
}

// QueueFamilies returns the available queue families.
func (pd *PhysicalDevice) QueueFamilies() []QueueFamily {
	result, _ := StartNew[[]QueueFamily]("compute-runtime.PhysicalDevice.QueueFamilies", nil,
		func(op *Operation[[]QueueFamily], rf *ResultFactory[[]QueueFamily]) *OperationResult[[]QueueFamily] {
			res := make([]QueueFamily, len(pd.queueFamilies))
			copy(res, pd.queueFamilies)
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// Limits returns the hardware limits.
func (pd *PhysicalDevice) Limits() DeviceLimits {
	result, _ := StartNew[DeviceLimits]("compute-runtime.PhysicalDevice.Limits", DeviceLimits{},
		func(op *Operation[DeviceLimits], rf *ResultFactory[DeviceLimits]) *OperationResult[DeviceLimits] {
			return rf.Generate(true, false, pd.limits)
		}).GetResult()
	return result
}

// Accelerator returns the underlying Layer 6 device (internal use).
func (pd *PhysicalDevice) Accelerator() devicesimulator.AcceleratorDevice {
	result, _ := StartNew[devicesimulator.AcceleratorDevice]("compute-runtime.PhysicalDevice.Accelerator", nil,
		func(op *Operation[devicesimulator.AcceleratorDevice], rf *ResultFactory[devicesimulator.AcceleratorDevice]) *OperationResult[devicesimulator.AcceleratorDevice] {
			return rf.Generate(true, false, pd.accelerator)
		}).GetResult()
	return result
}

// SupportsFeature checks if a feature is supported.
//
// Currently supported features:
//   - "fp32": 32-bit float (always true)
//   - "fp16": 16-bit float (always true)
//   - "unified_memory": CPU/GPU shared memory
//   - "transfer_queue": dedicated DMA engine
func (pd *PhysicalDevice) SupportsFeature(feature string) bool {
	result, _ := StartNew[bool]("compute-runtime.PhysicalDevice.SupportsFeature", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("feature", feature)
			switch feature {
			case "fp32", "fp16":
				return rf.Generate(true, false, true)
			case "unified_memory":
				return rf.Generate(true, false, pd.memoryProperties.IsUnified)
			case "transfer_queue":
				for _, qf := range pd.queueFamilies {
					if qf.QueueType == QueueTypeTransfer {
						return rf.Generate(true, false, true)
					}
				}
				return rf.Generate(true, false, false)
			default:
				return rf.Generate(true, false, false)
			}
		}).GetResult()
	return result
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
func (ld *LogicalDevice) PhysicalDevice() *PhysicalDevice {
	result, _ := StartNew[*PhysicalDevice]("compute-runtime.LogicalDevice.PhysicalDevice", nil,
		func(op *Operation[*PhysicalDevice], rf *ResultFactory[*PhysicalDevice]) *OperationResult[*PhysicalDevice] {
			return rf.Generate(true, false, ld.physical)
		}).GetResult()
	return result
}

// Queues returns command queues by type name ("compute", "transfer").
func (ld *LogicalDevice) Queues() map[string][]*CommandQueue {
	result, _ := StartNew[map[string][]*CommandQueue]("compute-runtime.LogicalDevice.Queues", nil,
		func(op *Operation[map[string][]*CommandQueue], rf *ResultFactory[map[string][]*CommandQueue]) *OperationResult[map[string][]*CommandQueue] {
			return rf.Generate(true, false, ld.queues)
		}).GetResult()
	return result
}

// MemoryManager returns the memory allocation manager.
func (ld *LogicalDevice) MemoryManager() *MemoryManager {
	result, _ := StartNew[*MemoryManager]("compute-runtime.LogicalDevice.MemoryManager", nil,
		func(op *Operation[*MemoryManager], rf *ResultFactory[*MemoryManager]) *OperationResult[*MemoryManager] {
			return rf.Generate(true, false, ld.memoryManager)
		}).GetResult()
	return result
}

// Stats returns runtime statistics.
func (ld *LogicalDevice) Stats() *RuntimeStats {
	result, _ := StartNew[*RuntimeStats]("compute-runtime.LogicalDevice.Stats", nil,
		func(op *Operation[*RuntimeStats], rf *ResultFactory[*RuntimeStats]) *OperationResult[*RuntimeStats] {
			return rf.Generate(true, false, ld.stats)
		}).GetResult()
	return result
}

// --- Factory methods ---

// CreateCommandBuffer creates a new command buffer.
func (ld *LogicalDevice) CreateCommandBuffer() *CommandBuffer {
	result, _ := StartNew[*CommandBuffer]("compute-runtime.LogicalDevice.CreateCommandBuffer", nil,
		func(op *Operation[*CommandBuffer], rf *ResultFactory[*CommandBuffer]) *OperationResult[*CommandBuffer] {
			return rf.Generate(true, false, NewCommandBuffer())
		}).GetResult()
	return result
}

// CreateShaderModule creates a shader module from options.
func (ld *LogicalDevice) CreateShaderModule(opts ShaderModuleOptions) *ShaderModule {
	result, _ := StartNew[*ShaderModule]("compute-runtime.LogicalDevice.CreateShaderModule", nil,
		func(op *Operation[*ShaderModule], rf *ResultFactory[*ShaderModule]) *OperationResult[*ShaderModule] {
			return rf.Generate(true, false, NewShaderModule(opts))
		}).GetResult()
	return result
}

// CreateDescriptorSetLayout creates a descriptor set layout.
func (ld *LogicalDevice) CreateDescriptorSetLayout(bindings []DescriptorBinding) *DescriptorSetLayout {
	result, _ := StartNew[*DescriptorSetLayout]("compute-runtime.LogicalDevice.CreateDescriptorSetLayout", nil,
		func(op *Operation[*DescriptorSetLayout], rf *ResultFactory[*DescriptorSetLayout]) *OperationResult[*DescriptorSetLayout] {
			return rf.Generate(true, false, NewDescriptorSetLayout(bindings))
		}).GetResult()
	return result
}

// CreatePipelineLayout creates a pipeline layout.
func (ld *LogicalDevice) CreatePipelineLayout(
	setLayouts []*DescriptorSetLayout,
	pushConstantSize int,
) *PipelineLayout {
	result, _ := StartNew[*PipelineLayout]("compute-runtime.LogicalDevice.CreatePipelineLayout", nil,
		func(op *Operation[*PipelineLayout], rf *ResultFactory[*PipelineLayout]) *OperationResult[*PipelineLayout] {
			return rf.Generate(true, false, NewPipelineLayout(setLayouts, pushConstantSize))
		}).GetResult()
	return result
}

// CreateComputePipeline creates a compute pipeline.
func (ld *LogicalDevice) CreateComputePipeline(
	shader *ShaderModule,
	layout *PipelineLayout,
) *Pipeline {
	result, _ := StartNew[*Pipeline]("compute-runtime.LogicalDevice.CreateComputePipeline", nil,
		func(op *Operation[*Pipeline], rf *ResultFactory[*Pipeline]) *OperationResult[*Pipeline] {
			return rf.Generate(true, false, NewPipeline(shader, layout))
		}).GetResult()
	return result
}

// CreateDescriptorSet creates a descriptor set from a layout.
func (ld *LogicalDevice) CreateDescriptorSet(layout *DescriptorSetLayout) *DescriptorSet {
	result, _ := StartNew[*DescriptorSet]("compute-runtime.LogicalDevice.CreateDescriptorSet", nil,
		func(op *Operation[*DescriptorSet], rf *ResultFactory[*DescriptorSet]) *OperationResult[*DescriptorSet] {
			return rf.Generate(true, false, NewDescriptorSet(layout))
		}).GetResult()
	return result
}

// CreateFence creates a fence for CPU<->GPU synchronization.
func (ld *LogicalDevice) CreateFence(signaled bool) *Fence {
	result, _ := StartNew[*Fence]("compute-runtime.LogicalDevice.CreateFence", nil,
		func(op *Operation[*Fence], rf *ResultFactory[*Fence]) *OperationResult[*Fence] {
			op.AddProperty("signaled", signaled)
			return rf.Generate(true, false, NewFence(signaled))
		}).GetResult()
	return result
}

// CreateSemaphore creates a semaphore for GPU queue<->queue synchronization.
func (ld *LogicalDevice) CreateSemaphore() *Semaphore {
	result, _ := StartNew[*Semaphore]("compute-runtime.LogicalDevice.CreateSemaphore", nil,
		func(op *Operation[*Semaphore], rf *ResultFactory[*Semaphore]) *OperationResult[*Semaphore] {
			return rf.Generate(true, false, NewSemaphore())
		}).GetResult()
	return result
}

// CreateEvent creates an event for fine-grained GPU-side signaling.
func (ld *LogicalDevice) CreateEvent() *Event {
	result, _ := StartNew[*Event]("compute-runtime.LogicalDevice.CreateEvent", nil,
		func(op *Operation[*Event], rf *ResultFactory[*Event]) *OperationResult[*Event] {
			return rf.Generate(true, false, NewEvent())
		}).GetResult()
	return result
}

// WaitIdle blocks until all queues finish all pending work.
func (ld *LogicalDevice) WaitIdle() {
	_, _ = StartNew[struct{}]("compute-runtime.LogicalDevice.WaitIdle", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for _, queueList := range ld.queues {
				for _, queue := range queueList {
					queue.WaitIdle()
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ResetDevice resets all device state.
func (ld *LogicalDevice) ResetDevice() {
	_, _ = StartNew[struct{}]("compute-runtime.LogicalDevice.ResetDevice", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			ld.accelerator.Reset()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
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
	result, _ := StartNew[*RuntimeInstance]("compute-runtime.NewRuntimeInstance", nil,
		func(op *Operation[*RuntimeInstance], rf *ResultFactory[*RuntimeInstance]) *OperationResult[*RuntimeInstance] {
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

			return rf.Generate(true, false, inst)
		}).GetResult()
	return result
}

// Version returns the runtime version string.
func (ri *RuntimeInstance) Version() string {
	result, _ := StartNew[string]("compute-runtime.RuntimeInstance.Version", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, ri.version)
		}).GetResult()
	return result
}

// EnumeratePhysicalDevices returns all available physical devices.
func (ri *RuntimeInstance) EnumeratePhysicalDevices() []*PhysicalDevice {
	result, _ := StartNew[[]*PhysicalDevice]("compute-runtime.RuntimeInstance.EnumeratePhysicalDevices", nil,
		func(op *Operation[[]*PhysicalDevice], rf *ResultFactory[[]*PhysicalDevice]) *OperationResult[[]*PhysicalDevice] {
			res := make([]*PhysicalDevice, len(ri.physicalDevices))
			copy(res, ri.physicalDevices)
			return rf.Generate(true, false, res)
		}).GetResult()
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
	result, _ := StartNew[*LogicalDevice]("compute-runtime.RuntimeInstance.CreateLogicalDevice", nil,
		func(op *Operation[*LogicalDevice], rf *ResultFactory[*LogicalDevice]) *OperationResult[*LogicalDevice] {
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

			return rf.Generate(true, false, &LogicalDevice{
				physical:      physicalDevice,
				accelerator:   accelerator,
				queues:        queues,
				memoryManager: memoryManager,
				stats:         stats,
			})
		}).GetResult()
	return result
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
