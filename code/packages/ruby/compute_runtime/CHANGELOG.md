# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial Ruby port of the compute-runtime package (from Python)
- **Device discovery**: `RuntimeInstance`, `PhysicalDevice`, `LogicalDevice` with support for nvidia, amd, google (TPU), intel, and apple (ANE) devices
- **Memory management**: `MemoryManager` with `allocate`, `free`, `map`, `unmap`, `flush`, `invalidate`; `Buffer` and `MappedMemory` types with bit-flag memory types (DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT, HOST_CACHED) and buffer usage flags (STORAGE, UNIFORM, TRANSFER_SRC, TRANSFER_DST, INDIRECT)
- **Command buffers**: `CommandBuffer` with state machine (initial -> recording -> recorded -> pending -> complete), compute commands (bind_pipeline, bind_descriptor_set, dispatch, dispatch_indirect, push_constants), transfer commands (copy_buffer, fill_buffer, update_buffer), and sync commands (pipeline_barrier, set_event, wait_event, reset_event)
- **Command queue**: `CommandQueue` with submit(), fence/semaphore support, execution trace collection
- **Pipeline**: `ShaderModule` (GPU-style with code or dataflow-style with operation), `DescriptorSetLayout`, `PipelineLayout`, `Pipeline`, `DescriptorSet` with binding validation
- **Synchronization**: `Fence` (CPU<->GPU), `Semaphore` (queue<->queue), `Event` (fine-grained GPU-side)
- **Validation layer**: `ValidationLayer` with command buffer state validation, dispatch validation, memory mapping validation, barrier tracking (read-after-write detection), and descriptor set validation
- **Protocol types**: All core data types using Ruby `Data.define` for frozen value objects
- Comprehensive test suite with 100+ tests covering all subsystems
