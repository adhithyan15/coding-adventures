# coding_adventures_compute_runtime

A Vulkan-inspired compute runtime for accelerator devices. Layer 5 of the accelerator computing stack.

## Overview

This package provides the core compute runtime that sits between the high-level scheduler and the low-level device simulator. It models the GPU programming model with:

- **Device discovery** -- enumerate physical devices (GPUs, TPUs, ANEs), create logical devices
- **Command buffers** -- record sequences of compute/transfer/sync commands, then submit
- **Memory management** -- allocate buffers with typed memory (device-local, host-visible), map/unmap for CPU access
- **Pipelines** -- shader modules + descriptor sets + pipeline layouts, following the Vulkan binding model
- **Synchronization** -- fences (CPU waits for GPU), semaphores (queue-to-queue), events (fine-grained GPU-side)
- **Validation layer** -- catches common GPU programming errors (use-after-free, missing barriers, wrong states)

## Architecture

```
RuntimeInstance
  └── PhysicalDevice (nvidia, amd, google, intel, apple)
        └── LogicalDevice
              ├── MemoryManager (allocate, free, map, unmap)
              ├── CommandQueue (submit command buffers)
              ├── Pipeline = ShaderModule + PipelineLayout
              ├── DescriptorSet (bind buffers to shader slots)
              └── Sync (Fence, Semaphore, Event)
```

## Stack Position

```
Layer 7: Scheduler / Graph Runtime
Layer 6: Device Simulator          <-- coding_adventures_device_simulator
Layer 5: Compute Runtime           <-- this package
Layer 4: (reserved)
Layer 3: (reserved)
Layer 2: (reserved)
Layer 1: GPU Core (ISA)            <-- coding_adventures_gpu_core
```

## Usage

```ruby
require "coding_adventures_compute_runtime"
include CodingAdventures

# 1. Create instance and discover devices
instance = ComputeRuntime::RuntimeInstance.new
devices = instance.enumerate_physical_devices
nvidia = devices.find { |d| d.vendor == "nvidia" }
device = instance.create_logical_device(nvidia)

# 2. Allocate memory
mm = device.memory_manager
staging = mm.allocate(256,
  ComputeRuntime::MemoryType::HOST_VISIBLE | ComputeRuntime::MemoryType::HOST_COHERENT,
  usage: ComputeRuntime::BufferUsage::TRANSFER_SRC)

# 3. Write data via mapped memory
mapped = mm.map(staging)
mapped.write(0, "\x42".b * 256)
mm.unmap(staging)

# 4. Create pipeline
shader = device.create_shader_module(
  code: [GpuCore.limm(0, 42.0), GpuCore.halt],
  local_size: [256, 1, 1]
)
ds_layout = device.create_descriptor_set_layout([
  ComputeRuntime::DescriptorBinding.new(binding: 0, type: "storage")
])
pl_layout = device.create_pipeline_layout([ds_layout])
pipeline = device.create_compute_pipeline(shader, pl_layout)

# 5. Record command buffer
cb = device.create_command_buffer
cb.begin
cb.cmd_bind_pipeline(pipeline)
cb.cmd_dispatch(4, 1, 1)
cb.end_recording

# 6. Submit with synchronization
queue = device.queues["compute"][0]
fence = device.create_fence
queue.submit([cb], fence: fence)
fence.wait  # CPU blocks until GPU finishes
```

## Dependencies

- `coding_adventures_gpu_core` -- GPU instruction set (limm, halt, etc.)
- `coding_adventures_device_simulator` -- device execution engine

## Development

```bash
bundle install
bundle exec rake test
```

## License

MIT
