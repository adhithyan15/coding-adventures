//! Vulkan Runtime Simulator -- the thinnest wrapper over Layer 5.
//!
//! # What is Vulkan?
//!
//! Vulkan is the Khronos Group's low-level, cross-platform GPU API. It's the
//! most explicit GPU API -- you manage everything: memory types, command buffer
//! recording, queue submission, synchronization barriers, descriptor set layouts.
//!
//! Because our Layer 5 compute runtime is already Vulkan-inspired, this
//! simulator is the **thinnest wrapper** of all six. It mainly adds:
//!
//! 1. Vulkan naming conventions (the `vk_` prefix on all methods)
//! 2. Vulkan-specific structures (VkBufferCreateInfo, VkSubmitInfo, etc.)
//! 3. VkResult return codes instead of Rust Result
//! 4. VkCommandPool for grouping command buffers
//!
//! # Why Vulkan is So Verbose
//!
//! Vulkan forces you to be explicit about everything because:
//!
//! 1. **No hidden allocations** -- you control every byte of memory
//! 2. **No implicit sync** -- you insert every barrier yourself
//! 3. **No automatic resource tracking** -- you free what you allocate
//! 4. **No driver guessing** -- you tell the driver exactly what you need

use std::collections::HashMap;

use compute_runtime::protocols::{
    DescriptorBinding, DeviceType, PipelineBarrier, PipelineStage,
};
use gpu_core::Instruction;

use crate::base::BaseSimulator;

// =========================================================================
// Vulkan enums
// =========================================================================

/// Vulkan function return codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VkResult {
    Success = 0,
    NotReady = 1,
    Timeout = 2,
    ErrorOutOfDeviceMemory = -3,
    ErrorDeviceLost = -4,
    ErrorInitializationFailed = -5,
}

/// Which pipeline type to bind -- compute or graphics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VkPipelineBindPoint {
    Compute,
}

/// Vulkan buffer usage flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VkBufferUsageFlagBits {
    StorageBuffer,
    UniformBuffer,
    TransferSrc,
    TransferDst,
}

/// Vulkan memory property flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VkMemoryPropertyFlagBits {
    DeviceLocal,
    HostVisible,
    HostCoherent,
    HostCached,
}

/// Whether a resource is used by one queue family or multiple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VkSharingMode {
    Exclusive,
    Concurrent,
}

// =========================================================================
// Vulkan create-info structures
// =========================================================================

/// Parameters for creating a VkBuffer.
#[derive(Debug, Clone)]
pub struct VkBufferCreateInfo {
    pub size: usize,
    pub usage: VkBufferUsageFlagBits,
    pub sharing_mode: VkSharingMode,
}

impl Default for VkBufferCreateInfo {
    fn default() -> Self {
        Self {
            size: 0,
            usage: VkBufferUsageFlagBits::StorageBuffer,
            sharing_mode: VkSharingMode::Exclusive,
        }
    }
}

/// Parameters for allocating device memory.
#[derive(Debug, Clone)]
pub struct VkMemoryAllocateInfo {
    pub size: usize,
    pub memory_type_index: usize,
}

/// Parameters for creating a shader module.
#[derive(Debug, Clone)]
pub struct VkShaderModuleCreateInfo {
    pub code: Option<Vec<Instruction>>,
}

/// Parameters for a queue submission.
#[derive(Debug, Default)]
pub struct VkSubmitInfo {
    pub command_buffer_ids: Vec<usize>,
}

/// Region to copy between buffers.
#[derive(Debug, Clone, Copy)]
pub struct VkBufferCopy {
    pub src_offset: usize,
    pub dst_offset: usize,
    pub size: usize,
}

/// One binding slot in a descriptor set layout.
#[derive(Debug, Clone)]
pub struct VkDescriptorSetLayoutBinding {
    pub binding: usize,
    pub descriptor_type: String,
    pub descriptor_count: usize,
}

impl Default for VkDescriptorSetLayoutBinding {
    fn default() -> Self {
        Self {
            binding: 0,
            descriptor_type: "storage".to_string(),
            descriptor_count: 1,
        }
    }
}

// =========================================================================
// Vulkan wrapper objects -- thin wrappers over Layer 5
// =========================================================================

/// Vulkan physical device wrapper.
#[derive(Debug, Clone)]
pub struct VkPhysicalDevice {
    pub index: usize,
    pub name: String,
    pub device_type: DeviceType,
    pub vendor: String,
}

/// Vulkan buffer wrapper (holds a buffer ID).
#[derive(Debug, Clone, Copy)]
pub struct VkBuffer {
    pub buffer_id: usize,
    pub size: usize,
}

/// Vulkan device memory (holds a buffer ID).
#[derive(Debug, Clone, Copy)]
pub struct VkDeviceMemory {
    pub buffer_id: usize,
}

/// Vulkan shader module (holds a shader module created on the device).
#[derive(Debug, Clone)]
pub struct VkShaderModule {
    pub code: Option<Vec<Instruction>>,
}

/// Vulkan pipeline (holds a pipeline ID).
#[derive(Debug, Clone, Copy)]
pub struct VkPipeline {
    pub pipeline_id: usize,
}

/// Vulkan descriptor set layout (holds binding info).
#[derive(Debug, Clone)]
pub struct VkDescriptorSetLayout {
    pub bindings: Vec<VkDescriptorSetLayoutBinding>,
}

/// Vulkan pipeline layout (holds descriptor set layout info).
#[derive(Debug, Clone)]
pub struct VkPipelineLayout {
    pub set_layouts: Vec<VkDescriptorSetLayout>,
    pub push_constant_size: usize,
}

/// Vulkan descriptor set (holds a descriptor set ID).
#[derive(Debug, Clone, Copy)]
pub struct VkDescriptorSet {
    pub set_id: usize,
}

/// Vulkan fence.
#[derive(Debug)]
pub struct VkFence {
    pub signaled: bool,
}

/// Vulkan semaphore.
#[derive(Debug)]
pub struct VkSemaphore {
    #[allow(dead_code)]
    id: usize,
}

/// Vulkan command pool -- groups command buffers.
pub struct VkCommandPool {
    command_buffer_ids: Vec<usize>,
}

/// Vulkan command buffer -- holds a CB ID.
#[derive(Debug, Clone, Copy)]
pub struct VkCommandBuffer {
    pub id: usize,
}

// =========================================================================
// VkDevice -- wraps LogicalDevice
// =========================================================================

/// Vulkan logical device -- the most verbose of all six simulators.
///
/// Every operation is a separate, explicit call with `vk_` prefix.
pub struct VkDevice {
    base: BaseSimulator,
    /// We store command buffers by ID so VkCommandBuffer can be a lightweight handle.
    command_buffers: HashMap<usize, compute_runtime::command_buffer::CommandBuffer>,
    next_cb_id: usize,
    /// Descriptor sets stored for later binding.
    descriptor_sets: HashMap<usize, compute_runtime::pipeline::DescriptorSet>,
}

impl VkDevice {
    fn new(base: BaseSimulator) -> Self {
        Self {
            base,
            command_buffers: HashMap::new(),
            next_cb_id: 0,
            descriptor_sets: HashMap::new(),
        }
    }

    /// Get a queue (in our simulator, returns a simple handle).
    pub fn vk_get_device_queue(&self, _family_index: usize, _queue_index: usize) -> VkResult {
        VkResult::Success
    }

    /// Create a command pool.
    pub fn vk_create_command_pool(&self) -> VkCommandPool {
        VkCommandPool {
            command_buffer_ids: Vec::new(),
        }
    }

    /// Allocate command buffers from a pool.
    pub fn vk_allocate_command_buffers(
        &mut self,
        pool: &mut VkCommandPool,
        count: usize,
    ) -> Vec<VkCommandBuffer> {
        let mut result = Vec::new();
        for _ in 0..count {
            let id = self.next_cb_id;
            self.next_cb_id += 1;
            let cb = self.base.device.create_command_buffer();
            self.command_buffers.insert(id, cb);
            pool.command_buffer_ids.push(id);
            result.push(VkCommandBuffer { id });
        }
        result
    }

    /// Allocate device memory.
    pub fn vk_allocate_memory(&mut self, alloc_info: &VkMemoryAllocateInfo) -> Result<VkDeviceMemory, String> {
        let buf_id = self.base.allocate_buffer(alloc_info.size)?;
        Ok(VkDeviceMemory { buffer_id: buf_id })
    }

    /// Create a buffer.
    pub fn vk_create_buffer(&mut self, create_info: &VkBufferCreateInfo) -> Result<VkBuffer, String> {
        let buf_id = self.base.allocate_buffer(create_info.size)?;
        Ok(VkBuffer {
            buffer_id: buf_id,
            size: create_info.size,
        })
    }

    /// Bind memory to a buffer (no-op in our simulator).
    pub fn vk_bind_buffer_memory(&self, _buffer: &VkBuffer, _memory: &VkDeviceMemory, _offset: usize) {
        // In our simulator, buffers are already backed by memory.
    }

    /// Map device memory for CPU access.
    pub fn vk_map_memory(&mut self, memory: &VkDeviceMemory, offset: usize, size: usize) -> Result<Vec<u8>, String> {
        let mm = self.base.device.memory_manager_mut();
        mm.invalidate(memory.buffer_id, 0, 0)?;
        let mapped = mm.map(memory.buffer_id)?;
        let data = mapped.read(offset, size)?;
        Ok(data)
    }

    /// Unmap device memory.
    pub fn vk_unmap_memory(&mut self, memory: &VkDeviceMemory) -> Result<(), String> {
        let mm = self.base.device.memory_manager_mut();
        // Only unmap if currently mapped
        let buf = mm.get_buffer(memory.buffer_id)?;
        if buf.mapped {
            mm.unmap(memory.buffer_id)?;
        }
        Ok(())
    }

    /// Write data to mapped memory.
    pub fn vk_write_mapped_memory(&mut self, memory: &VkDeviceMemory, offset: usize, data: &[u8]) -> Result<(), String> {
        let mm = self.base.device.memory_manager_mut();
        {
            let mut mapped = mm.map(memory.buffer_id)?;
            mapped.write(offset, data)?;
        }
        mm.unmap(memory.buffer_id)?;
        Ok(())
    }

    /// Create a shader module.
    pub fn vk_create_shader_module(&self, create_info: &VkShaderModuleCreateInfo) -> VkShaderModule {
        VkShaderModule {
            code: create_info.code.clone(),
        }
    }

    /// Create a descriptor set layout.
    pub fn vk_create_descriptor_set_layout(
        &self,
        bindings: &[VkDescriptorSetLayoutBinding],
    ) -> VkDescriptorSetLayout {
        VkDescriptorSetLayout {
            bindings: bindings.to_vec(),
        }
    }

    /// Create a pipeline layout.
    pub fn vk_create_pipeline_layout(
        &self,
        set_layouts: &[VkDescriptorSetLayout],
        push_constant_size: usize,
    ) -> VkPipelineLayout {
        VkPipelineLayout {
            set_layouts: set_layouts.to_vec(),
            push_constant_size,
        }
    }

    /// Create a compute pipeline.
    pub fn vk_create_compute_pipeline(
        &mut self,
        shader_module: &VkShaderModule,
        layout: &VkPipelineLayout,
    ) -> VkPipeline {
        let bindings: Vec<DescriptorBinding> = layout
            .set_layouts
            .iter()
            .flat_map(|sl| {
                sl.bindings
                    .iter()
                    .map(|b| DescriptorBinding::new(b.binding).with_type(&b.descriptor_type))
            })
            .collect();

        let shader = self.base.device.create_shader_module(
            shader_module.code.clone(),
            "",
            "main",
            (1, 1, 1),
        );
        let ds_layout = self.base.device.create_descriptor_set_layout(bindings);
        let pl_layout = self
            .base
            .device
            .create_pipeline_layout(vec![ds_layout], layout.push_constant_size);
        let pipeline_id = self.base.device.create_compute_pipeline(shader, pl_layout);

        VkPipeline { pipeline_id }
    }

    /// Allocate a descriptor set.
    pub fn vk_allocate_descriptor_set(
        &mut self,
        layout: &VkDescriptorSetLayout,
    ) -> VkDescriptorSet {
        let bindings: Vec<DescriptorBinding> = layout
            .bindings
            .iter()
            .map(|b| DescriptorBinding::new(b.binding).with_type(&b.descriptor_type))
            .collect();
        let ds_layout = self.base.device.create_descriptor_set_layout(bindings);
        let ds = self.base.device.create_descriptor_set(ds_layout);
        let id = ds.set_id();
        self.descriptor_sets.insert(id, ds);
        VkDescriptorSet { set_id: id }
    }

    /// Write a buffer binding to a descriptor set.
    pub fn vk_update_descriptor_set(
        &mut self,
        set: &VkDescriptorSet,
        binding: usize,
        buffer: &VkBuffer,
    ) -> Result<(), String> {
        let ds = self
            .descriptor_sets
            .get_mut(&set.set_id)
            .ok_or("Descriptor set not found")?;
        ds.write(binding, buffer.buffer_id)
    }

    /// Create a fence.
    pub fn vk_create_fence(&self, signaled: bool) -> VkFence {
        VkFence { signaled }
    }

    /// Create a semaphore.
    pub fn vk_create_semaphore(&self) -> VkSemaphore {
        let sem = self.base.device.create_semaphore();
        VkSemaphore {
            id: sem.semaphore_id(),
        }
    }

    /// Wait for fences.
    pub fn vk_wait_for_fences(&self, fences: &[&VkFence], wait_all: bool) -> VkResult {
        for f in fences {
            if f.signaled {
                if !wait_all {
                    return VkResult::Success;
                }
            } else if wait_all {
                return VkResult::NotReady;
            }
        }
        VkResult::Success
    }

    /// Reset fences to unsignaled state.
    pub fn vk_reset_fences(&mut self, fences: &mut [&mut VkFence]) {
        for f in fences {
            f.signaled = false;
        }
    }

    /// Begin command buffer recording.
    pub fn vk_begin_command_buffer(&mut self, cb: &VkCommandBuffer) -> Result<(), String> {
        let inner = self
            .command_buffers
            .get_mut(&cb.id)
            .ok_or("Command buffer not found")?;
        inner.begin()
    }

    /// End command buffer recording.
    pub fn vk_end_command_buffer(&mut self, cb: &VkCommandBuffer) -> Result<(), String> {
        let inner = self
            .command_buffers
            .get_mut(&cb.id)
            .ok_or("Command buffer not found")?;
        inner.end()
    }

    /// Bind a pipeline in a command buffer.
    pub fn vk_cmd_bind_pipeline(&mut self, cb: &VkCommandBuffer, pipeline: &VkPipeline) -> Result<(), String> {
        let inner = self
            .command_buffers
            .get_mut(&cb.id)
            .ok_or("Command buffer not found")?;
        inner.cmd_bind_pipeline(pipeline.pipeline_id)
    }

    /// Bind a descriptor set in a command buffer.
    pub fn vk_cmd_bind_descriptor_sets(
        &mut self,
        cb: &VkCommandBuffer,
        sets: &[&VkDescriptorSet],
    ) -> Result<(), String> {
        let inner = self
            .command_buffers
            .get_mut(&cb.id)
            .ok_or("Command buffer not found")?;
        for ds in sets {
            inner.cmd_bind_descriptor_set(ds.set_id)?;
        }
        Ok(())
    }

    /// Dispatch compute work.
    pub fn vk_cmd_dispatch(
        &mut self,
        cb: &VkCommandBuffer,
        x: usize,
        y: usize,
        z: usize,
    ) -> Result<(), String> {
        let inner = self
            .command_buffers
            .get_mut(&cb.id)
            .ok_or("Command buffer not found")?;
        inner.cmd_dispatch(x, y, z)
    }

    /// Copy between buffers.
    pub fn vk_cmd_copy_buffer(
        &mut self,
        cb: &VkCommandBuffer,
        src: &VkBuffer,
        dst: &VkBuffer,
        regions: &[VkBufferCopy],
    ) -> Result<(), String> {
        let inner = self
            .command_buffers
            .get_mut(&cb.id)
            .ok_or("Command buffer not found")?;
        for r in regions {
            inner.cmd_copy_buffer(src.buffer_id, dst.buffer_id, r.size, r.src_offset, r.dst_offset)?;
        }
        Ok(())
    }

    /// Fill buffer with a value.
    pub fn vk_cmd_fill_buffer(
        &mut self,
        cb: &VkCommandBuffer,
        buffer: &VkBuffer,
        value: u8,
        offset: usize,
        size: usize,
    ) -> Result<(), String> {
        let inner = self
            .command_buffers
            .get_mut(&cb.id)
            .ok_or("Command buffer not found")?;
        inner.cmd_fill_buffer(buffer.buffer_id, value, offset, size)
    }

    /// Insert a pipeline barrier.
    pub fn vk_cmd_pipeline_barrier(
        &mut self,
        cb: &VkCommandBuffer,
        src_stage: PipelineStage,
        dst_stage: PipelineStage,
    ) -> Result<(), String> {
        let inner = self
            .command_buffers
            .get_mut(&cb.id)
            .ok_or("Command buffer not found")?;
        let barrier = PipelineBarrier {
            src_stage,
            dst_stage,
            ..PipelineBarrier::default()
        };
        inner.cmd_pipeline_barrier(&barrier)
    }

    /// Submit command buffers to the queue.
    pub fn vk_queue_submit(
        &mut self,
        cb_handles: &[&VkCommandBuffer],
        fence: Option<&mut VkFence>,
    ) -> Result<VkResult, String> {
        for &cb_handle in cb_handles {
            // Take the CB out, submit it, put it back
            let mut inner_cb = self
                .command_buffers
                .remove(&cb_handle.id)
                .ok_or("Command buffer not found")?;

            let mut real_fence = self.base.device.create_fence(false);
            self.base.device.submit(
                "compute",
                0,
                &mut [&mut inner_cb],
                &mut [],
                &mut [],
                Some(&mut real_fence),
            )?;

            self.command_buffers.insert(cb_handle.id, inner_cb);
        }

        if let Some(f) = fence {
            f.signaled = true;
        }

        Ok(VkResult::Success)
    }

    /// Wait for all work to complete.
    pub fn vk_device_wait_idle(&self) {
        self.base.device.wait_idle();
    }

    /// Free device memory.
    pub fn vk_free_memory(&mut self, memory: &VkDeviceMemory) -> Result<(), String> {
        self.base.device.memory_manager_mut().free(memory.buffer_id)
    }
}

// =========================================================================
// VkInstance -- the Vulkan entry point
// =========================================================================

/// Vulkan instance -- the entry point for device discovery.
///
/// Unlike CUDA (which auto-selects NVIDIA) or Metal (which auto-selects
/// Apple), Vulkan gives you all devices and lets you choose.
pub struct VkInstance {
    base: BaseSimulator,
}

impl VkInstance {
    /// Create a new Vulkan instance.
    pub fn new() -> Result<Self, String> {
        let base = BaseSimulator::new(None, None)?;
        Ok(Self { base })
    }

    /// Enumerate all physical devices.
    pub fn vk_enumerate_physical_devices(&self) -> Vec<VkPhysicalDevice> {
        self.base
            .instance
            .enumerate_physical_devices()
            .iter()
            .enumerate()
            .map(|(i, pd)| VkPhysicalDevice {
                index: i,
                name: pd.name().to_string(),
                device_type: pd.device_type(),
                vendor: pd.vendor().to_string(),
            })
            .collect()
    }

    /// Create a logical device from a physical device.
    pub fn vk_create_device(&self, physical_device: &VkPhysicalDevice) -> Result<VkDevice, String> {
        let base = BaseSimulator::new(None, Some(&physical_device.vendor))?;
        Ok(VkDevice::new(base))
    }
}
