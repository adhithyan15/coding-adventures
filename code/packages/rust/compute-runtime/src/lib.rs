//! # Compute Runtime -- Layer 5 of the accelerator computing stack.
//!
//! A low-level Vulkan-inspired compute runtime that provides the software
//! infrastructure between user-facing APIs (CUDA, OpenCL, Metal, Vulkan)
//! and the hardware device simulators (Layer 6).
//!
//! ## Architecture
//!
//! ```text
//! RuntimeInstance
//! +-- enumerate_physical_devices() -> PhysicalDevice[]
//! +-- create_logical_device() -> LogicalDevice
//!     +-- queues: CommandQueue[]
//!     +-- memory_manager: MemoryManager
//!     +-- create_command_buffer() -> CommandBuffer
//!     +-- create_compute_pipeline() -> Pipeline
//!     +-- create_fence() -> Fence
//!     +-- create_semaphore() -> Semaphore
//! ```
//!
//! ## Quick Start
//!
//! ```text
//! use compute_runtime::instance::RuntimeInstance;
//! use compute_runtime::protocols::{MemoryType, BufferUsage};
//! use gpu_core::opcodes::{limm, halt};
//!
//! // 1. Discover devices
//! let instance = RuntimeInstance::new(None);
//! let devices = instance.enumerate_physical_devices();
//! let nvidia_idx = devices.iter().position(|d| d.vendor() == "nvidia").unwrap();
//!
//! // 2. Create logical device
//! let mut device = instance.create_logical_device(nvidia_idx, None).unwrap();
//!
//! // 3. Create pipeline, record commands, submit...
//! ```

pub mod protocols;
pub mod memory;
pub mod command_buffer;
pub mod command_queue;
pub mod instance;
pub mod pipeline;
pub mod sync;
pub mod validation;

// Re-export commonly used types.
pub use protocols::{
    AccessFlags, BufferUsage, CommandBufferState, DeviceType, MemoryType,
    PipelineStage, QueueType, RuntimeEventType,
    BufferBarrier, DescriptorBinding, DeviceLimits, MemoryBarrier, MemoryHeap,
    MemoryProperties, PipelineBarrier, QueueFamily, RecordedCommand, RuntimeStats,
    RuntimeTrace,
};
pub use memory::{Buffer, MappedMemory, MemoryManager};
pub use command_buffer::CommandBuffer;
pub use command_queue::CommandQueue;
pub use instance::{LogicalDevice, PhysicalDevice, RuntimeInstance};
pub use pipeline::{
    DescriptorSet, DescriptorSetLayout, Pipeline, PipelineLayout, ShaderModule,
};
pub use sync::{Event, Fence, Semaphore};
pub use validation::{ValidationError, ValidationLayer};
