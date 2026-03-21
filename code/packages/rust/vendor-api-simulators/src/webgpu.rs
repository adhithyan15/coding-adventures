//! WebGPU Runtime Simulator -- safe, browser-first GPU programming.
//!
//! # What is WebGPU?
//!
//! WebGPU is the modern web GPU API, designed to run safely in browsers.
//! It sits on top of Vulkan (Linux/Windows/Android), Metal (macOS/iOS),
//! or D3D12 (Windows), providing a safe, portable abstraction.
//!
//! # Key Simplifications Over Vulkan
//!
//! 1. **Single queue** -- `device.queue` is all you get. No queue families.
//! 2. **Automatic barriers** -- no manual pipeline barriers.
//! 3. **No memory types** -- just usage flags. The runtime picks optimal memory.
//! 4. **Always validated** -- every operation is checked.
//! 5. **Immutable command buffers** -- once `finish()` is called, frozen.
//!
//! # The WebGPU Object Hierarchy
//!
//! ```text
//! GPU (navigator.gpu in browsers)
//! +-- GPUAdapter (represents a physical device)
//!     +-- GPUDevice (the usable handle)
//!         +-- device.queue (GPUQueue -- single queue!)
//!         +-- create_buffer() -> GPUBuffer
//!         +-- create_shader_module() -> GPUShaderModule
//!         +-- create_compute_pipeline() -> GPUComputePipeline
//!         +-- create_bind_group() -> GPUBindGroup
//!         +-- create_command_encoder() -> GPUCommandEncoder
//!             +-- begin_compute_pass() -> GPUComputePassEncoder
//!                 +-- set_pipeline()
//!                 +-- set_bind_group()
//!                 +-- dispatch_workgroups()
//!                 +-- end()
//!             +-- finish() -> GPUCommandBuffer (frozen!)
//! ```

use std::collections::HashMap;

use compute_runtime::protocols::DescriptorBinding;
use gpu_core::Instruction;

use crate::base::BaseSimulator;

// =========================================================================
// WebGPU flags
// =========================================================================

/// WebGPU buffer usage flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBufferUsage {
    MapRead,
    MapWrite,
    CopySrc,
    CopyDst,
    Storage,
    Uniform,
}

/// WebGPU buffer map modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuMapMode {
    Read,
    Write,
}

// =========================================================================
// WebGPU descriptor types
// =========================================================================

/// Parameters for creating a GPUBuffer.
pub struct GpuBufferDescriptor {
    pub size: usize,
    pub usage: GpuBufferUsage,
    pub mapped_at_creation: bool,
}

impl Default for GpuBufferDescriptor {
    fn default() -> Self {
        Self {
            size: 0,
            usage: GpuBufferUsage::Storage,
            mapped_at_creation: false,
        }
    }
}

// =========================================================================
// WebGPU wrapper objects
// =========================================================================

/// A WebGPU buffer -- memory on the device.
///
/// WebGPU buffers don't expose memory types. You specify usage flags,
/// and the runtime picks the optimal memory type.
pub struct GpuBuffer {
    pub buffer_id: usize,
    pub size: usize,
    pub usage: GpuBufferUsage,
    mapped: bool,
    mapped_data: Option<Vec<u8>>,
    destroyed: bool,
}

impl GpuBuffer {
    /// Map the buffer for CPU access (simulated as synchronous).
    pub fn map_async(&mut self, mm: &mut compute_runtime::memory::MemoryManager, _mode: GpuMapMode) -> Result<(), String> {
        if self.destroyed {
            return Err("Cannot map a destroyed buffer".to_string());
        }
        mm.invalidate(self.buffer_id, 0, 0)?;
        let data = {
            let mapped = mm.map(self.buffer_id)?;
            mapped.read(0, self.size)?
        };
        mm.unmap(self.buffer_id)?;
        self.mapped_data = Some(data);
        self.mapped = true;
        Ok(())
    }

    /// Get a view of the mapped buffer data.
    pub fn get_mapped_range(&self, offset: usize, size: usize) -> Result<Vec<u8>, String> {
        let data = self
            .mapped_data
            .as_ref()
            .ok_or("Buffer is not mapped. Call map_async() first.")?;
        let actual_size = if size == 0 { data.len() } else { size };
        if offset + actual_size > data.len() {
            return Err("Mapped range out of bounds".to_string());
        }
        Ok(data[offset..offset + actual_size].to_vec())
    }

    /// Unmap the buffer, syncing data back to the device.
    pub fn unmap(&mut self, mm: &mut compute_runtime::memory::MemoryManager) -> Result<(), String> {
        if !self.mapped {
            return Err("Buffer is not mapped".to_string());
        }
        if let Some(ref data) = self.mapped_data {
            {
                let mut mapped = mm.map(self.buffer_id)?;
                mapped.write(0, data)?;
            }
            mm.unmap(self.buffer_id)?;
        }
        self.mapped = false;
        self.mapped_data = None;
        Ok(())
    }

    /// Destroy this buffer, releasing its memory.
    pub fn destroy(&mut self, mm: &mut compute_runtime::memory::MemoryManager) -> Result<(), String> {
        if !self.destroyed {
            mm.free(self.buffer_id)?;
            self.destroyed = true;
        }
        Ok(())
    }

    /// Check if the buffer has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }
}

/// A frozen WebGPU command buffer -- immutable after `finish()`.
pub struct GpuCommandBuffer {
    cb_commands: Vec<GpuRecordedCommand>,
}

pub(crate) enum GpuRecordedCommand {
    BindPipeline(usize),
    BindDescriptorSet(usize),
    Dispatch(usize, usize, usize),
    CopyBuffer {
        src_id: usize,
        dst_id: usize,
        size: usize,
        src_offset: usize,
        dst_offset: usize,
    },
}

/// A WebGPU compute pass encoder -- records compute commands.
pub struct GpuComputePassEncoder {
    pipeline_id: Option<usize>,
    bind_groups: HashMap<usize, usize>, // index -> ds_id
    commands: Vec<GpuRecordedCommand>,
}

impl GpuComputePassEncoder {
    /// Set the compute pipeline for this pass.
    pub fn set_pipeline(&mut self, pipeline: &GpuComputePipeline) {
        self.pipeline_id = Some(pipeline.pipeline_id);
    }

    /// Set a bind group at the given index.
    pub fn set_bind_group(&mut self, _index: usize, bind_group: &GpuBindGroup) {
        self.bind_groups.insert(_index, bind_group.ds_id);
    }

    /// Dispatch compute workgroups.
    pub fn dispatch_workgroups(&mut self, x: usize, y: usize, z: usize) -> Result<(), String> {
        let pid = self
            .pipeline_id
            .ok_or("No pipeline set")?;
        self.commands.push(GpuRecordedCommand::BindPipeline(pid));
        let mut sorted_keys: Vec<usize> = self.bind_groups.keys().copied().collect();
        sorted_keys.sort();
        for k in sorted_keys {
            self.commands
                .push(GpuRecordedCommand::BindDescriptorSet(self.bind_groups[&k]));
        }
        self.commands
            .push(GpuRecordedCommand::Dispatch(x, y, z));
        Ok(())
    }

    /// End this compute pass.
    pub fn end(self) -> Vec<GpuRecordedCommand> {
        self.commands
    }
}

/// A WebGPU command encoder -- builds a GPUCommandBuffer.
pub struct GpuCommandEncoder {
    commands: Vec<GpuRecordedCommand>,
}

impl GpuCommandEncoder {
    /// Begin a compute pass.
    pub fn begin_compute_pass(&self) -> GpuComputePassEncoder {
        GpuComputePassEncoder {
            pipeline_id: None,
            bind_groups: HashMap::new(),
            commands: Vec::new(),
        }
    }

    /// Merge commands from a finished compute pass.
    pub fn end_compute_pass(&mut self, pass: GpuComputePassEncoder) {
        self.commands.extend(pass.end());
    }

    /// Copy data between buffers.
    pub fn copy_buffer_to_buffer(
        &mut self,
        source: &GpuBuffer,
        source_offset: usize,
        destination: &GpuBuffer,
        destination_offset: usize,
        size: usize,
    ) {
        self.commands.push(GpuRecordedCommand::CopyBuffer {
            src_id: source.buffer_id,
            dst_id: destination.buffer_id,
            size,
            src_offset: source_offset,
            dst_offset: destination_offset,
        });
    }

    /// Finish recording and produce a frozen command buffer.
    pub fn finish(self) -> GpuCommandBuffer {
        GpuCommandBuffer {
            cb_commands: self.commands,
        }
    }
}

/// A WebGPU compute pipeline.
pub struct GpuComputePipeline {
    pub pipeline_id: usize,
    bind_group_layout_bindings: Vec<Vec<usize>>,
}

impl GpuComputePipeline {
    /// Get bind group layout info at an index (returns binding indices).
    pub fn get_bind_group_layout_bindings(&self, index: usize) -> Option<&[usize]> {
        self.bind_group_layout_bindings.get(index).map(|v| v.as_slice())
    }
}

/// A WebGPU bind group -- wraps Layer 5 DescriptorSet.
pub struct GpuBindGroup {
    pub ds_id: usize,
}

// =========================================================================
// GPUDevice -- the main WebGPU device
// =========================================================================

/// A WebGPU device -- the main entry point for GPU programming.
///
/// Provides `queue` for submission and factory methods for all GPU resources.
pub struct GpuDevice {
    base: BaseSimulator,
    /// Descriptor sets stored for later binding.
    descriptor_sets: HashMap<usize, compute_runtime::pipeline::DescriptorSet>,
}

impl GpuDevice {
    fn new(base: BaseSimulator) -> Self {
        Self {
            base,
            descriptor_sets: HashMap::new(),
        }
    }

    /// Create a buffer.
    pub fn create_buffer(&mut self, descriptor: &GpuBufferDescriptor) -> Result<GpuBuffer, String> {
        let buf_id = self.base.allocate_buffer(descriptor.size)?;
        let mut gpu_buf = GpuBuffer {
            buffer_id: buf_id,
            size: descriptor.size,
            usage: descriptor.usage,
            mapped: false,
            mapped_data: None,
            destroyed: false,
        };

        if descriptor.mapped_at_creation {
            gpu_buf.map_async(self.base.device.memory_manager_mut(), GpuMapMode::Write)?;
        }

        Ok(gpu_buf)
    }

    /// Create a compute pipeline.
    pub fn create_compute_pipeline(
        &mut self,
        code: Option<Vec<Instruction>>,
        binding_indices: &[usize],
    ) -> GpuComputePipeline {
        let shader = self.base.device.create_shader_module(
            code,
            "",
            "main",
            (1, 1, 1),
        );
        let descriptor_bindings: Vec<DescriptorBinding> = binding_indices
            .iter()
            .map(|&b| DescriptorBinding::new(b))
            .collect();
        let ds_layout = self
            .base
            .device
            .create_descriptor_set_layout(descriptor_bindings);
        let pl_layout = self
            .base
            .device
            .create_pipeline_layout(vec![ds_layout], 0);
        let pipeline_id = self.base.device.create_compute_pipeline(shader, pl_layout);

        GpuComputePipeline {
            pipeline_id,
            bind_group_layout_bindings: vec![binding_indices.to_vec()],
        }
    }

    /// Create a bind group.
    pub fn create_bind_group(
        &mut self,
        entries: &[(usize, &GpuBuffer)],
    ) -> Result<GpuBindGroup, String> {
        let binding_indices: Vec<usize> = entries.iter().map(|(idx, _)| *idx).collect();
        let descriptor_bindings: Vec<DescriptorBinding> = binding_indices
            .iter()
            .map(|&b| DescriptorBinding::new(b))
            .collect();
        let ds_layout = self
            .base
            .device
            .create_descriptor_set_layout(descriptor_bindings);
        let mut ds = self.base.device.create_descriptor_set(ds_layout);
        for (idx, buf) in entries {
            ds.write(*idx, buf.buffer_id)?;
        }
        let id = ds.set_id();
        self.descriptor_sets.insert(id, ds);
        Ok(GpuBindGroup { ds_id: id })
    }

    /// Create a command encoder.
    pub fn create_command_encoder(&self) -> GpuCommandEncoder {
        GpuCommandEncoder {
            commands: Vec::new(),
        }
    }

    /// Submit command buffers to the queue.
    pub fn queue_submit(&mut self, command_buffers: &[GpuCommandBuffer]) -> Result<(), String> {
        for gpu_cb in command_buffers {
            let mut cb = self.base.device.create_command_buffer();
            cb.begin()?;
            for cmd in &gpu_cb.cb_commands {
                match cmd {
                    GpuRecordedCommand::BindPipeline(id) => {
                        cb.cmd_bind_pipeline(*id)?;
                    }
                    GpuRecordedCommand::BindDescriptorSet(id) => {
                        cb.cmd_bind_descriptor_set(*id)?;
                    }
                    GpuRecordedCommand::Dispatch(x, y, z) => {
                        cb.cmd_dispatch(*x, *y, *z)?;
                    }
                    GpuRecordedCommand::CopyBuffer {
                        src_id,
                        dst_id,
                        size,
                        src_offset,
                        dst_offset,
                    } => {
                        cb.cmd_copy_buffer(*src_id, *dst_id, *size, *src_offset, *dst_offset)?;
                    }
                }
            }
            cb.end()?;
            let mut fence = self.base.device.create_fence(false);
            self.base.device.submit(
                "compute",
                0,
                &mut [&mut cb],
                &mut [],
                &mut [],
                Some(&mut fence),
            )?;
        }
        Ok(())
    }

    /// Write data to a buffer (convenience method).
    pub fn queue_write_buffer(
        &mut self,
        buffer: &GpuBuffer,
        buffer_offset: usize,
        data: &[u8],
    ) -> Result<(), String> {
        let mm = self.base.device.memory_manager_mut();
        {
            let mut mapped = mm.map(buffer.buffer_id)?;
            mapped.write(buffer_offset, data)?;
        }
        mm.unmap(buffer.buffer_id)?;
        Ok(())
    }

    /// Read data from a buffer.
    pub fn read_buffer(&mut self, buffer: &GpuBuffer) -> Result<Vec<u8>, String> {
        let mm = self.base.device.memory_manager_mut();
        mm.invalidate(buffer.buffer_id, 0, 0)?;
        let data = {
            let mapped = mm.map(buffer.buffer_id)?;
            mapped.read(0, buffer.size)?
        };
        mm.unmap(buffer.buffer_id)?;
        Ok(data)
    }

    /// Get a mutable reference to the memory manager.
    pub fn memory_manager_mut(&mut self) -> &mut compute_runtime::memory::MemoryManager {
        self.base.device.memory_manager_mut()
    }

    /// Destroy the device and release all resources.
    pub fn destroy(&self) {
        self.base.device.wait_idle();
    }
}

// =========================================================================
// GPUAdapter -- physical device wrapper
// =========================================================================

/// A WebGPU adapter -- represents a physical GPU.
pub struct GpuAdapter {
    pub name: String,
    pub vendor: String,
}

impl GpuAdapter {
    /// Request a device from this adapter.
    pub fn request_device(&self) -> Result<GpuDevice, String> {
        let base = BaseSimulator::new(None, Some(&self.vendor))?;
        Ok(GpuDevice::new(base))
    }
}

// =========================================================================
// GPU -- the top-level WebGPU entry point
// =========================================================================

/// The WebGPU entry point -- like `navigator.gpu` in browsers.
pub struct Gpu {
    base: BaseSimulator,
}

impl Gpu {
    /// Create the GPU entry point.
    pub fn new() -> Result<Self, String> {
        let base = BaseSimulator::new(None, None)?;
        Ok(Self { base })
    }

    /// Request a GPU adapter.
    pub fn request_adapter(&self) -> Result<GpuAdapter, String> {
        let devices = self.base.instance.enumerate_physical_devices();
        if devices.is_empty() {
            return Err("No GPU adapters available".to_string());
        }
        Ok(GpuAdapter {
            name: devices[0].name().to_string(),
            vendor: devices[0].vendor().to_string(),
        })
    }
}
