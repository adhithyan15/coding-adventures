// VulkanBlas -- explicit Vulkan BLAS backend.
//
// # How VulkanBlas Works
//
// This backend wraps the Vulkan API from Layer 4. Vulkan is the most verbose
// GPU API -- you explicitly manage everything: buffer creation, memory
// allocation, binding, mapping, and unmapping.
//
// For each BLAS operation, we allocate device memory via the VkInstance's
// underlying memory manager, write data via the map/write/unmap cycle,
// and read it back the same way.
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
// The reward is maximum performance and predictability -- the driver does
// exactly what you say, nothing more.
package backends

import (
	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
	cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
	vas "github.com/adhithyan15/coding-adventures/code/packages/go/vendor-api-simulators"
)

// =========================================================================
// VulkanBlas -- maximum control GPU acceleration
// =========================================================================

// VulkanBlas wraps the Vulkan API for BLAS operations. Since Vulkan is the
// thinnest wrapper over Layer 5, we use the VkInstance's underlying memory
// manager directly for the most explicit memory management.
//
// Usage:
//
//	vk, _ := NewVulkanBlas()
//	result, _ := vk.Sgemm(NoTrans, NoTrans, 1.0, A, B, 0.0, C)
type VulkanBlas struct {
	gpuBase
	instance *vas.VkInstance
	device   *vas.VkDevice
}

// NewVulkanBlas creates a new Vulkan BLAS backend via the full Vulkan setup ceremony.
//
// The Vulkan initialization sequence:
//
//  1. Create a VkInstance (the Vulkan entry point)
//  2. Enumerate physical devices
//  3. Create a logical device from the first physical device
func NewVulkanBlas() (*VulkanBlas, error) {
	instance, err := vas.NewVkInstance()
	if err != nil {
		return nil, err
	}
	physDevices := instance.VkEnumeratePhysicalDevices()
	device := instance.VkCreateDevice(physDevices[0])
	vb := &VulkanBlas{instance: instance, device: device}
	vb.gpuBase = newGpuBase(vb)
	return vb, nil
}

// Name returns the backend identifier.
func (v *VulkanBlas) Name() string { return "vulkan" }

// DeviceName returns a human-readable device name.
func (v *VulkanBlas) DeviceName() string { return "Vulkan Device" }

// =========================================================================
// gpuMemory implementation -- Vulkan explicit memory management
// =========================================================================

// upload allocates device memory and writes data via the memory manager's
// map/write/unmap cycle.
//
// We bypass VkAllocateMemory and use the instance's underlying MemoryManager
// directly, because VkMapMemory returns a snapshot copy (not a live reference),
// which means writes to the returned slice don't persist. Using the memory
// manager's Map/Write/Unmap cycle ensures data is properly written.
func (v *VulkanBlas) upload(data []byte) (interface{}, error) {
	buf, err := v.instance.MemoryManager.Allocate(
		len(data), vas.DefaultMemType(), vas.DefaultUsage(),
	)
	if err != nil {
		return nil, err
	}
	mapped, err := v.instance.MemoryManager.Map(buf)
	if err != nil {
		return nil, err
	}
	if err := mapped.Write(0, data); err != nil {
		return nil, err
	}
	if err := v.instance.MemoryManager.Unmap(buf); err != nil {
		return nil, err
	}
	return buf, nil
}

// download reads data from device memory via invalidate + GetBufferData.
func (v *VulkanBlas) download(handle interface{}, size int) ([]byte, error) {
	buf := handle.(*cr.Buffer)
	if err := v.instance.MemoryManager.Invalidate(buf, 0, 0); err != nil {
		return nil, err
	}
	data := v.instance.MemoryManager.GetBufferData(buf.BufferID)
	result := make([]byte, size)
	copy(result, data[:size])
	return result, nil
}

// free releases device memory via the memory manager.
func (v *VulkanBlas) free(handle interface{}) error {
	buf := handle.(*cr.Buffer)
	return v.instance.MemoryManager.Free(buf)
}

// Compile-time checks that VulkanBlas implements both interfaces.
var _ blas.BlasBackend = (*VulkanBlas)(nil)
var _ blas.MlBlasBackend = (*VulkanBlas)(nil)
