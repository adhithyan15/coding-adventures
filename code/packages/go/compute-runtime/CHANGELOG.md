# Changelog

All notable changes to the compute-runtime Go package will be documented here.

## [0.1.0] - 2026-03-19

### Added

- Initial Go port of the Python compute-runtime package
- `protocols.go`: All enum types (DeviceType, QueueType, MemoryType, BufferUsage,
  PipelineStage, AccessFlags, CommandBufferState, RuntimeEventType) using const
  iota and bit flags where appropriate
- `protocols.go`: Data structures (QueueFamily, DeviceLimits, MemoryHeap,
  MemoryProperties, DescriptorBinding, RecordedCommand, MemoryBarrier,
  BufferBarrier, PipelineBarrierDesc, RuntimeTrace, RuntimeStats)
- `instance.go`: RuntimeInstance with device discovery and default device
  creation for all 5 vendors (NVIDIA, AMD, Google, Intel, Apple)
- `instance.go`: PhysicalDevice (read-only hardware description) and
  LogicalDevice (usable handle with factory methods)
- `memory.go`: MemoryManager with Allocate, Free, Map, Unmap, Flush, Invalidate
- `memory.go`: Buffer and MappedMemory types with bounds-checked read/write
- `command_buffer.go`: CommandBuffer with full state machine (INITIAL ->
  RECORDING -> RECORDED -> PENDING -> COMPLETE) and all Cmd* methods
- `command_queue.go`: CommandQueue with Submit including semaphore wait/signal
  and fence signaling; executes dispatch, copy, fill, update, barrier commands
- `pipeline.go`: ShaderModule (GPU-style and dataflow-style), DescriptorSetLayout,
  PipelineLayout, Pipeline, DescriptorSet with Write/GetBuffer
- `sync.go`: Fence, Semaphore, Event synchronization primitives
- `validation.go`: ValidationLayer with checks for command buffer state,
  dispatch validation, memory mapping, buffer usage, barrier tracking,
  and descriptor set compatibility
- 128 tests passing at 93.0% coverage
