# vendor-api-simulators

Rust implementations of six GPU vendor API simulators, built as thin wrappers over the `compute-runtime` (Layer 5). Each simulator faithfully reproduces the programming model and vocabulary of a real GPU API while delegating all actual work to the shared compute runtime.

## What This Package Does

Real GPU programming requires choosing a vendor API -- CUDA, OpenCL, Metal, Vulkan, WebGPU, or OpenGL. Each API has a completely different programming model, but underneath they all do the same things: find a GPU, allocate memory, record commands, and dispatch compute work.

This package implements all six APIs as Rust structs that wrap a shared `BaseSimulator`. Think of it like six different restaurant fronts (CUDA Grill, Metal Bistro, Vulkan Steakhouse...) that all share the same kitchen in the back.

## The Six Simulators

| Simulator | Style | Key Abstraction | Entry Point |
|-----------|-------|-----------------|-------------|
| **CUDA** | Implicit/NVIDIA-only | Streams + kernel launch | `CudaRuntime` |
| **OpenCL** | Portable/event-driven | Platform/context/queue | `ClContext` |
| **Metal** | Apple/encoder model | Command encoders | `MtlDevice` |
| **Vulkan** | Ultra-explicit | Create-info structs | `VkInstance` |
| **WebGPU** | Safe/browser-first | Bind groups + encoders | `Gpu` |
| **OpenGL** | Legacy state machine | Integer handles + bind | `GlContext` |

## Architecture

```
┌─────────────────────────────────────────────────────┐
│              Vendor API Simulators (Layer 3)         │
│  ┌──────┐ ┌──────┐ ┌─────┐ ┌──────┐ ┌─────┐ ┌────┐│
│  │ CUDA │ │OpenCL│ │Metal│ │Vulkan│ │WebGPU│ │ GL ││
│  └──┬───┘ └──┬───┘ └──┬──┘ └──┬───┘ └──┬───┘ └─┬──┘│
│     │        │        │       │        │       │    │
│     └────────┴────────┴───┬───┴────────┴───────┘    │
│                           │                         │
│                    BaseSimulator                    │
│                           │                         │
└───────────────────────────┼─────────────────────────┘
                            │
                ┌───────────┴───────────┐
                │   compute-runtime     │
                │      (Layer 5)        │
                └───────────────────────┘
```

## Usage

```rust
use vendor_api_simulators::cuda::*;

// CUDA style -- simple and implicit
let mut cuda = CudaRuntime::new().unwrap();
let ptr = cuda.malloc(1024).unwrap();
cuda.memcpy_host_to_device(&ptr, &data).unwrap();
cuda.launch_kernel(&kernel, Dim3::new(4, 1, 1), Dim3::new(256, 1, 1), &[&ptr]).unwrap();
cuda.memcpy_device_to_host(&ptr, &mut output).unwrap();
cuda.free(ptr).unwrap();
```

```rust
use vendor_api_simulators::vulkan::*;

// Vulkan style -- ultra-explicit
let instance = VkInstance::new().unwrap();
let mut device = instance.create_device(0).unwrap();
let buffer = device.create_buffer(&VkBufferCreateInfo { size: 1024 }).unwrap();
let memory = device.allocate_memory(&VkMemoryAllocateInfo { size: 1024 }).unwrap();
device.bind_buffer_memory(buffer, memory).unwrap();
// ... record command buffers, submit, wait ...
```

## Design Decisions

- **Composition over inheritance**: Each simulator owns a `BaseSimulator` struct rather than inheriting from a base class (Rust doesn't have inheritance).
- **ID-based handles**: The compute runtime uses `usize` IDs for buffers, pipelines, and descriptor sets. Each simulator wraps these in API-appropriate types.
- **Deferred command recording**: WebGPU and Vulkan record commands into intermediate structs, then replay them into real command buffers during submission.
- **State machine for OpenGL**: `GlContext` uses HashMaps to track bound buffers, active programs, and shader state, faithfully reproducing OpenGL's global state model.

## Testing

```bash
cargo test -p vendor-api-simulators
```

186 integration tests covering:
- 34 CUDA tests
- 30 OpenCL tests
- 30 Metal tests
- 30 Vulkan tests
- 30 WebGPU tests
- 28 OpenGL tests
- 4 cross-API tests

## How It Fits in the Stack

This package is Layer 3 in the educational GPU computing stack:

- **Layer 1**: `fp-arithmetic` -- IEEE 754 floating-point
- **Layer 2**: `gpu-core` -- GPU hardware simulation
- **Layer 3**: `vendor-api-simulators` -- **this package**
- **Layer 4**: `device-simulator` -- device abstraction
- **Layer 5**: `compute-runtime` -- Vulkan-inspired runtime
