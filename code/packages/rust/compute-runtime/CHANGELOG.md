# Changelog

All notable changes to compute-runtime (Rust) will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial Rust port from the Python compute-runtime package
- **protocols.rs**: All shared types ported using Rust idioms
  - `DeviceType`, `QueueType`, `PipelineStage`, `CommandBufferState`, `RuntimeEventType` as enums
  - `MemoryType`, `BufferUsage`, `AccessFlags` using the `bitflags` crate
  - `DeviceLimits`, `MemoryHeap`, `MemoryProperties`, `QueueFamily` as structs
  - `RecordedCommand` with `CommandArg` enum for type-safe argument storage
  - `MemoryBarrier`, `BufferBarrier`, `PipelineBarrier` structs
  - `RuntimeTrace` and `RuntimeStats` for observability
- **instance.rs**: Device discovery and management
  - `RuntimeInstance` with default devices (NVIDIA, AMD, Google TPU, Intel, Apple ANE)
  - `PhysicalDevice` with feature queries (fp32, fp16, unified_memory, transfer_queue)
  - `LogicalDevice` with factory methods and queue management
  - `AcceleratorFactory` for lazy device creation
- **memory.rs**: Typed memory management
  - `Buffer` struct with memory type and usage flags
  - `MappedMemory` with lifetime-safe CPU access
  - `MemoryManager` wrapping Layer 6 malloc/free with tracking
  - Peak allocation tracking, flush/invalidate operations
- **command_buffer.rs**: Vulkan-style command recording
  - Full state machine (Initial -> Recording -> Recorded -> Pending -> Complete)
  - Compute commands: bind_pipeline, bind_descriptor_set, push_constants, dispatch, dispatch_indirect
  - Transfer commands: copy_buffer, fill_buffer, update_buffer
  - Sync commands: pipeline_barrier, set_event, wait_event, reset_event
- **command_queue.rs**: FIFO command submission and execution
  - Semaphore wait/signal during submission
  - Fence signaling on completion
  - Command dispatch through device-simulator Layer 6
  - Runtime trace generation for all operations
- **pipeline.rs**: Pipeline and descriptor management
  - `ShaderModule` supporting GPU-style (instruction code) and dataflow-style (operation name)
  - `DescriptorSetLayout`, `PipelineLayout`, `Pipeline`
  - `DescriptorSet` with validated buffer binding
- **sync.rs**: Three synchronization primitives
  - `Fence` for CPU-GPU synchronization
  - `Semaphore` for GPU queue-to-queue synchronization
  - `Event` for fine-grained GPU-side signaling
- **validation.rs**: Development-time validation layer
  - Command buffer state validation
  - Dispatch validation (pipeline bound, positive dimensions)
  - Memory validation (HOST_VISIBLE for mapping, freed detection)
  - Buffer usage flag checking
  - Write-after-read barrier warnings
- 67 unit tests across all modules
- 46 integration tests covering full pipeline execution
- Literate programming style with detailed doc comments
