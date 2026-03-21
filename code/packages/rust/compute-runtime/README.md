# compute-runtime (Rust)

**Layer 5 of the accelerator computing stack** -- a Vulkan-inspired compute runtime that sits between user-facing APIs (CUDA, OpenCL, Metal) and the hardware device simulators (Layer 6).

## What It Does

The compute runtime manages the full lifecycle of GPU/TPU/NPU workloads:

- **Device discovery**: Enumerate available accelerators (NVIDIA, AMD, Google TPU, Intel, Apple ANE)
- **Memory management**: Typed allocations with DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT flags
- **Command recording**: Vulkan-style record-then-submit command buffers
- **Pipeline management**: Shader modules, descriptor sets, compute pipelines
- **Synchronization**: Fences (CPU-GPU), Semaphores (GPU-GPU), Events (fine-grained)
- **Validation**: Development-time error checking for common GPU programming mistakes

## Architecture

```
RuntimeInstance
+-- enumerate_physical_devices() -> PhysicalDevice[]
+-- create_logical_device() -> LogicalDevice
    +-- queues: CommandQueue[]
    +-- memory_manager: MemoryManager
    +-- create_command_buffer() -> CommandBuffer
    +-- create_compute_pipeline() -> Pipeline
    +-- create_fence() -> Fence
    +-- create_semaphore() -> Semaphore
```

## Quick Start

```rust
use compute_runtime::instance::RuntimeInstance;
use compute_runtime::protocols::{MemoryType, BufferUsage};
use gpu_core::opcodes::{limm, halt};

// 1. Discover devices
let instance = RuntimeInstance::new(None);

// 2. Create logical device (by index)
let mut device = instance.create_logical_device(0, None).unwrap();

// 3. Allocate memory
let buf_id = device.memory_manager_mut().allocate(
    256,
    MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE,
    BufferUsage::STORAGE,
).unwrap();

// 4. Create pipeline
let shader = device.create_shader_module(
    Some(vec![limm(0, 42.0), halt()]),
    "", "main", (32, 1, 1),
);
let ds_layout = device.create_descriptor_set_layout(vec![]);
let pl_layout = device.create_pipeline_layout(vec![ds_layout], 0);
let pipeline_id = device.create_compute_pipeline(shader, pl_layout);

// 5. Record and submit
let mut cb = device.create_command_buffer();
cb.begin().unwrap();
cb.cmd_bind_pipeline(pipeline_id).unwrap();
cb.cmd_dispatch(1, 1, 1).unwrap();
cb.end().unwrap();

let mut fence = device.create_fence(false);
device.submit("compute", 0, &mut [&mut cb], &mut [], &mut [], Some(&mut fence)).unwrap();
assert!(fence.wait(None));
```

## Module Structure

| Module | Purpose |
|--------|---------|
| `protocols` | Shared types: enums, bitflags, data structures |
| `instance` | Device discovery (RuntimeInstance, PhysicalDevice, LogicalDevice) |
| `memory` | Buffer allocation, mapping, staging (MemoryManager, Buffer, MappedMemory) |
| `command_buffer` | Command recording (begin/end/dispatch/copy/barrier) |
| `command_queue` | Command submission and execution against hardware |
| `pipeline` | ShaderModule, DescriptorSet, Pipeline |
| `sync` | Fence, Semaphore, Event |
| `validation` | ValidationLayer for catching programming errors |

## Dependencies

- `device-simulator` -- Layer 6 hardware simulators
- `gpu-core` -- Layer 9 ISA (instruction types)
- `bitflags` -- Ergonomic flag types for MemoryType, BufferUsage, AccessFlags

## How It Fits in the Stack

```
User code:     y = alpha * x + y
     |
API layer:     cudaMalloc / vkAllocateMemory / MTLBuffer  (Layer 4, future)
     |
Runtime:       CommandBuffer, Queue, Fence, Memory types   (THIS LAYER)
     |
Hardware:      NvidiaGPU.launch_kernel, .step(), .run()    (Layer 6)
```

## Build & Test

```bash
cargo test -p compute-runtime -- --nocapture
```
