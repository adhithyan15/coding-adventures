//! VulkanBlas -- explicit Vulkan BLAS backend.
//!
//! # How VulkanBlas Works
//!
//! This backend wraps the Vulkan API from Layer 4. Vulkan is the most verbose
//! GPU API -- you explicitly manage everything: buffer creation, memory
//! allocation, binding, mapping, and unmapping.
//!
//! For each BLAS operation, we allocate VkDeviceMemory, write data via the
//! `vk_write_mapped_memory` call, and read it back with `vk_map_memory`.

use vendor_api_simulators::vulkan::{VkInstance, VkDevice, VkMemoryAllocateInfo};

use super::gpu_base::GpuBlasBackend;

/// Vulkan BLAS backend -- wraps VkDevice from Layer 4.
///
/// # Vulkan BLAS -- Maximum Control GPU Acceleration
///
/// Vulkan forces you to be explicit about everything:
/// - Buffer creation with usage flags
/// - Memory allocation with property flags
/// - Explicit map/unmap for data transfer
///
/// The reward is maximum performance and predictability.
pub struct VulkanBlas {
    _instance: VkInstance,
    device: VkDevice,
}

impl VulkanBlas {
    /// Create a new Vulkan BLAS backend.
    pub fn new() -> Result<Self, String> {
        let instance = VkInstance::new()?;
        let physical_devices = instance.vk_enumerate_physical_devices();
        let device = instance.vk_create_device(&physical_devices[0])?;
        Ok(Self {
            _instance: instance,
            device,
        })
    }
}

impl GpuBlasBackend for VulkanBlas {
    fn gpu_name(&self) -> &str {
        "vulkan"
    }

    fn gpu_device_name(&self) -> String {
        "Vulkan Device".to_string()
    }

    fn upload(&mut self, data: &[u8]) -> Result<usize, String> {
        let alloc_info = VkMemoryAllocateInfo {
            size: data.len(),
            memory_type_index: 0,
        };
        let memory = self.device.vk_allocate_memory(&alloc_info)?;
        self.device.vk_write_mapped_memory(&memory, 0, data)?;
        Ok(memory.buffer_id)
    }

    fn download(&mut self, handle: usize, size: usize) -> Result<Vec<u8>, String> {
        let memory = vendor_api_simulators::vulkan::VkDeviceMemory {
            buffer_id: handle,
        };
        let data = self.device.vk_map_memory(&memory, 0, size)?;
        self.device.vk_unmap_memory(&memory)?;
        Ok(data)
    }

    fn free(&mut self, handle: usize) -> Result<(), String> {
        let memory = vendor_api_simulators::vulkan::VkDeviceMemory {
            buffer_id: handle,
        };
        self.device.vk_free_memory(&memory)
    }
}
