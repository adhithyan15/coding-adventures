// Package vendorapisimulators implements Layer 3 of the accelerator computing stack --
// six vendor API simulators that provide the programming interfaces developers
// actually use to program GPUs and accelerators.
//
// # What Are Vendor API Simulators?
//
// When a CUDA developer writes cudaMalloc(), the CUDA runtime translates that
// into explicit memory allocation, memory type selection, and buffer creation.
// When a WebGPU developer calls device.createBuffer(), the browser's WebGPU
// implementation does the same translation internally.
//
// This package provides those translation layers. Each simulator wraps the
// compute-runtime (Layer 5) with a different vendor's vocabulary:
//
//	CUDA      -- NVIDIA's implicit, "just launch it" model
//	OpenCL    -- Khronos Group's portable, event-based model
//	Metal     -- Apple's unified memory, command encoder model
//	Vulkan    -- Khronos Group's ultra-explicit, verbose model
//	WebGPU    -- The safe, browser-first GPU API
//	OpenGL    -- The legacy global state machine
//
// # The Kitchen Analogy
//
// Think of it like building six different restaurant fronts (CUDA Grill, Metal
// Bistro, Vulkan Steakhouse...) that all share the same kitchen in the back.
// The kitchen is our compute runtime (Layer 5). The restaurant menus look
// completely different, but the same chefs cook the same food.
//
// # Base Simulator
//
// BaseVendorSimulator is the shared foundation. Every GPU API, no matter how
// different its surface looks, needs to do the same things underneath:
//
//  1. Find a GPU                  --> RuntimeInstance
//  2. Create a usable handle      --> LogicalDevice
//  3. Get a queue for submission   --> CommandQueue
//  4. Manage memory                --> MemoryManager
//
// Each simulator embeds BaseVendorSimulator and adds vendor-specific vocabulary.
package vendorapisimulators

import (
	"fmt"

	cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
)

// =========================================================================
// BaseVendorSimulator -- the shared foundation for all six simulators
// =========================================================================

// BaseVendorSimulator provides the common infrastructure that every vendor
// API simulator needs: device discovery, queue setup, and memory management.
//
// # What Every Subclass Gets
//
//   - Instance:         *cr.RuntimeInstance  (Layer 5 entry point)
//   - PhysicalDevices:  []*cr.PhysicalDevice (all available hardware)
//   - PhysicalDevice:   *cr.PhysicalDevice   (the selected device)
//   - LogicalDevice:    *cr.LogicalDevice    (the usable handle)
//   - ComputeQueue:     *cr.CommandQueue     (for submitting work)
//   - MemoryManager:    *cr.MemoryManager    (for allocating memory)
//
// # Device Selection
//
// Different APIs have different preferences:
//   - CUDA always wants an NVIDIA GPU (vendorHint="nvidia")
//   - Metal always wants an Apple device (vendorHint="apple")
//   - OpenCL, Vulkan, WebGPU, OpenGL are cross-vendor
//
// The selectDevice method handles a four-pass search:
//
//	Pass 1: Match both vendor AND device type
//	Pass 2: Match vendor only
//	Pass 3: Match device type only
//	Pass 4: Take whatever is available
type BaseVendorSimulator struct {
	Instance        *cr.RuntimeInstance
	PhysicalDevices []*cr.PhysicalDevice
	PhysicalDevice  *cr.PhysicalDevice
	LogicalDevice   *cr.LogicalDevice
	ComputeQueue    *cr.CommandQueue
	MemoryManager   *cr.MemoryManager
}

// InitBase initializes the base simulator with device discovery and setup.
//
// # Initialization Flow
//
//  1. Create runtime instance (discovers all hardware)
//  2. Enumerate all physical devices
//  3. Select the best matching device (vendor preference)
//  4. Create a logical device (the usable handle)
//  5. Get a compute queue for submitting work
//  6. Get the memory manager for allocations
//
// deviceType can be nil for "any type". vendorHint can be empty for "any vendor".
func InitBase(deviceType *cr.DeviceType, vendorHint string) (*BaseVendorSimulator, error) {
	base := &BaseVendorSimulator{}

	// Step 1: Create the runtime instance
	base.Instance = cr.NewRuntimeInstance(nil)

	// Step 2: Enumerate all physical devices
	base.PhysicalDevices = base.Instance.EnumeratePhysicalDevices()
	if len(base.PhysicalDevices) == 0 {
		return nil, fmt.Errorf("no physical devices available")
	}

	// Step 3: Select the best matching device
	base.PhysicalDevice = selectDevice(base.PhysicalDevices, deviceType, vendorHint)

	// Step 4: Create a logical device
	base.LogicalDevice = base.Instance.CreateLogicalDevice(base.PhysicalDevice, nil)

	// Step 5: Get the compute queue
	queues := base.LogicalDevice.Queues()
	if computeQueues, ok := queues["compute"]; ok && len(computeQueues) > 0 {
		base.ComputeQueue = computeQueues[0]
	} else {
		return nil, fmt.Errorf("no compute queue available on device")
	}

	// Step 6: Get the memory manager
	base.MemoryManager = base.LogicalDevice.MemoryManager()

	return base, nil
}

// selectDevice picks the best matching device from enumerated physical devices.
//
// # Selection Strategy
//
// The strategy is a four-pass filter, from most specific to least:
//
//	Pass 1: Match both vendorHint AND deviceType (if both given)
//	Pass 2: Match vendorHint only
//	Pass 3: Match deviceType only
//	Pass 4: Take the first device (any will do)
//
// This ensures that:
//   - CUDARuntime(vendorHint="nvidia") gets an NVIDIA GPU
//   - MTLDevice(vendorHint="apple") gets an Apple device
//   - VulkanInstance() gets whatever is available
func selectDevice(
	devices []*cr.PhysicalDevice,
	deviceType *cr.DeviceType,
	vendorHint string,
) *cr.PhysicalDevice {
	// Pass 1: Match both vendor and type
	if vendorHint != "" && deviceType != nil {
		for _, dev := range devices {
			if dev.Vendor() == vendorHint && dev.DeviceType() == *deviceType {
				return dev
			}
		}
	}

	// Pass 2: Match vendor only
	if vendorHint != "" {
		for _, dev := range devices {
			if dev.Vendor() == vendorHint {
				return dev
			}
		}
	}

	// Pass 3: Match device type only
	if deviceType != nil {
		for _, dev := range devices {
			if dev.DeviceType() == *deviceType {
				return dev
			}
		}
	}

	// Pass 4: Take whatever is available
	return devices[0]
}

// CreateAndSubmitCB creates a command buffer, records commands via a callback,
// submits it, and waits for completion.
//
// # The "Immediate Execution" Pattern
//
// APIs like CUDA and OpenGL present an "immediate" execution model where each
// API call appears to execute right away. Under the hood, they still use command
// buffers -- they just hide them from you.
//
// This method implements that pattern:
//
//  1. Create a new command buffer
//  2. Begin recording
//  3. Call recordFn(cb) to record whatever commands the caller wants
//  4. End recording
//  5. Submit to the queue with a fence
//  6. Wait for the fence to signal (synchronous completion)
//  7. Return the command buffer (for inspection/debugging)
//
// The recordFn receives a *cr.CommandBuffer in RECORDING state.
// If queue is nil, uses the base's ComputeQueue.
func (b *BaseVendorSimulator) CreateAndSubmitCB(
	recordFn func(*cr.CommandBuffer) error,
	queue *cr.CommandQueue,
) (*cr.CommandBuffer, error) {
	targetQueue := queue
	if targetQueue == nil {
		targetQueue = b.ComputeQueue
	}

	// Create and begin recording
	cb := b.LogicalDevice.CreateCommandBuffer()
	if err := cb.Begin(); err != nil {
		return nil, fmt.Errorf("failed to begin command buffer: %w", err)
	}

	// Let the caller record whatever commands they need
	if err := recordFn(cb); err != nil {
		return nil, fmt.Errorf("failed to record commands: %w", err)
	}

	// End recording and submit
	if err := cb.End(); err != nil {
		return nil, fmt.Errorf("failed to end command buffer: %w", err)
	}

	fence := b.LogicalDevice.CreateFence(false)
	_, err := targetQueue.Submit([]*cr.CommandBuffer{cb}, &cr.SubmitOptions{Fence: fence})
	if err != nil {
		return nil, fmt.Errorf("failed to submit command buffer: %w", err)
	}
	fence.Wait(nil)

	return cb, nil
}

// DefaultMemType returns the standard memory type used across all simulators:
// DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT.
//
// This combination allows both GPU compute and CPU read/write access, which
// is needed for our simulation where we want to verify results from tests.
func DefaultMemType() cr.MemoryType {
	return cr.MemoryTypeDeviceLocal | cr.MemoryTypeHostVisible | cr.MemoryTypeHostCoherent
}

// DefaultUsage returns the standard buffer usage flags:
// STORAGE | TRANSFER_SRC | TRANSFER_DST.
//
// This combination allows the buffer to be used as a shader storage buffer,
// and as both source and destination of copy operations.
func DefaultUsage() cr.BufferUsage {
	return cr.BufferUsageStorage | cr.BufferUsageTransferSrc | cr.BufferUsageTransferDst
}
