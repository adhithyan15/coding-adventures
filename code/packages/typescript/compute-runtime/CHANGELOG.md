# Changelog

## 0.1.0 (2026-03-19)

### Added
- Initial TypeScript implementation ported from Python compute-runtime package
- **RuntimeInstance** for device discovery and logical device creation
- **PhysicalDevice** with memory properties, queue families, device limits
- **LogicalDevice** with command queues, memory manager, factory methods
- **MemoryManager** with typed allocations (DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT)
- **Buffer** and **MappedMemory** for CPU-GPU data transfer
- **CommandBuffer** with record-then-submit model (Vulkan-style state machine)
- **CommandQueue** with FIFO submission, semaphore/fence support, execution tracing
- **Pipeline**, **ShaderModule**, **DescriptorSetLayout**, **DescriptorSet**
- **Fence** (CPU-GPU), **Semaphore** (GPU-GPU), **Event** (fine-grained GPU sync)
- **PipelineBarrier** with memory and buffer barrier support
- **ValidationLayer** for catching programming errors (state, dispatch, memory, barrier, descriptor)
- **RuntimeTrace** and **RuntimeStats** for observability
- Support for all 5 device architectures (NVIDIA, AMD, Google TPU, Intel, Apple ANE)
- Zero-copy unified memory pattern for Apple devices
- Staging buffer upload pattern for discrete GPUs
- Full test suite: 181 tests, 96%+ line coverage
