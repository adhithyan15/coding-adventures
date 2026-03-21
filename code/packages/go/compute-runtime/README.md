# compute-runtime (Go)

Layer 5 of the accelerator computing stack -- a Vulkan-inspired compute runtime
that sits between user-facing APIs (CUDA, OpenCL, Metal, Vulkan) and the
hardware device simulators (Layer 6).

## What It Does

The compute runtime manages the full lifecycle of GPU/TPU/NPU work:

```
User code:     y = alpha * x + y
     |
API layer:     cudaMalloc / vkAllocateMemory / MTLBuffer  (Layer 4, future)
     |
Runtime:       CommandBuffer, Queue, Fence, Memory types   (THIS LAYER)
     |
Hardware:      NvidiaGPU.LaunchKernel, .Step(), .Run()     (Layer 6)
```

## Architecture

```
RuntimeInstance
+-- EnumeratePhysicalDevices() -> []*PhysicalDevice
+-- CreateLogicalDevice() -> *LogicalDevice
    +-- Queues: map[string][]*CommandQueue
    +-- MemoryManager: *MemoryManager
    +-- CreateCommandBuffer() -> *CommandBuffer
    +-- CreateComputePipeline() -> *Pipeline
    +-- CreateFence() -> *Fence
    +-- CreateSemaphore() -> *Semaphore
```

## Quick Start

```go
package main

import (
    cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
    gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

func main() {
    // 1. Discover devices
    instance := cr.NewRuntimeInstance(nil)
    devices := instance.EnumeratePhysicalDevices()
    nvidia := devices[0]

    // 2. Create logical device
    device := instance.CreateLogicalDevice(nvidia, nil)
    queue := device.Queues()["compute"][0]
    mm := device.MemoryManager()

    // 3. Allocate buffers
    memType := cr.MemoryTypeDeviceLocal | cr.MemoryTypeHostVisible
    buf, _ := mm.Allocate(256, memType, cr.BufferUsageStorage)

    // 4. Create pipeline
    shader := device.CreateShaderModule(cr.ShaderModuleOptions{
        Code:      []gpucore.Instruction{gpucore.Halt()},
        LocalSize: [3]int{1, 1, 1},
    })
    dsLayout := device.CreateDescriptorSetLayout(nil)
    plLayout := device.CreatePipelineLayout([]*cr.DescriptorSetLayout{dsLayout}, 0)
    pipeline := device.CreateComputePipeline(shader, plLayout)

    // 5. Record and submit commands
    cb := device.CreateCommandBuffer()
    cb.Begin()
    cb.CmdBindPipeline(pipeline)
    cb.CmdDispatch(1, 1, 1)
    cb.End()

    fence := device.CreateFence(false)
    queue.Submit([]*cr.CommandBuffer{cb}, &cr.SubmitOptions{Fence: fence})
    fence.Wait(nil)

    // 6. Cleanup
    mm.Free(buf)
}
```

## Modules

| File | Description |
|------|-------------|
| `protocols.go` | Types, enums (const iota), interfaces, data structures |
| `instance.go` | RuntimeInstance, PhysicalDevice, LogicalDevice |
| `memory.go` | MemoryManager, Buffer, MappedMemory |
| `command_buffer.go` | CommandBuffer with all Cmd* methods |
| `command_queue.go` | CommandQueue, Submit |
| `pipeline.go` | ShaderModule, Pipeline, DescriptorSet |
| `sync.go` | Fence, Semaphore, Event |
| `validation.go` | ValidationLayer |

## Key Design Decisions

- **Vulkan-inspired**: Models at Vulkan's explicit level so higher-level APIs
  (CUDA, Metal, OpenCL) can be built on top.
- **Error returns**: All fallible operations return errors instead of panicking.
- **Bit flags**: MemoryType and BufferUsage use bit flags (`1 << iota`) for
  combinable properties, matching Vulkan's flag patterns.
- **ID counters**: Package-level counters generate unique IDs for all objects
  (command buffers, pipelines, fences, etc.).

## Dependencies

- `device-simulator` (Layer 6) -- the hardware device interface
- `gpu-core` -- generic ISA instruction types

## Building & Testing

```bash
go test ./... -v -cover
```
