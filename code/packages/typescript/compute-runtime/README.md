# @coding-adventures/compute-runtime

Layer 5 of the accelerator computing stack — a low-level Vulkan-inspired compute runtime that provides the software infrastructure between user-facing APIs (CUDA, OpenCL, Metal, Vulkan) and the hardware device simulators (Layer 6).

## What is this?

This package implements a **Vulkan-inspired compute runtime** that manages the full lifecycle of GPU/TPU/NPU compute work: device discovery, memory allocation, command recording, queue submission, synchronization, and pipeline management.

```
Layer 9:  gpu-core (one core, one instruction at a time)
    |
Layer 8:  parallel-execution-engine (warps, wavefronts, systolic arrays)
    |
Layer 7:  compute-unit (SM, CU, MXU, XeCore, ANECore)
    |
Layer 6:  device-simulator (complete devices)
    |
Layer 5:  compute-runtime (THIS PACKAGE)
    |
    +-- RuntimeInstance        -- device discovery (like vkEnumeratePhysicalDevices)
    +-- LogicalDevice          -- queue + memory + factory (like VkDevice)
    +-- CommandBuffer          -- record-then-submit (like VkCommandBuffer)
    +-- CommandQueue           -- FIFO submission (like VkQueue)
    +-- MemoryManager          -- typed allocations (like VkDeviceMemory)
    +-- Pipeline               -- shader + layout (like VkPipeline)
    +-- Fence/Semaphore/Event  -- sync primitives (like Vulkan sync)
    +-- ValidationLayer        -- error detection (like VK_LAYER_KHRONOS_validation)
```

## Architecture

The runtime follows the Vulkan model closely:

| Concept | Vulkan Equivalent | Description |
|---------|-------------------|-------------|
| RuntimeInstance | VkInstance | Entry point, enumerates physical devices |
| PhysicalDevice | VkPhysicalDevice | Hardware capabilities, memory properties |
| LogicalDevice | VkDevice | Queues, memory manager, object factory |
| CommandBuffer | VkCommandBuffer | Records commands: dispatch, copy, barrier |
| CommandQueue | VkQueue | Submits recorded command buffers for execution |
| MemoryManager | VkDeviceMemory | Allocate, map, free typed buffers |
| Pipeline | VkPipeline | Shader module + descriptor layout |
| Fence | VkFence | CPU-GPU synchronization |
| Semaphore | VkSemaphore | GPU-GPU queue synchronization |
| Event | VkEvent | Fine-grained GPU synchronization |
| ValidationLayer | Validation Layers | Catches programming errors |

## Quick Start

```typescript
import { RuntimeInstance, MemoryType, BufferUsage, PipelineStage, AccessFlags, makePipelineBarrier } from "@coding-adventures/compute-runtime";
import { limm, halt } from "@coding-adventures/gpu-core";

// 1. Create instance and discover devices
const instance = new RuntimeInstance();
const physical = instance.enumeratePhysicalDevices().find(d => d.vendor === "nvidia")!;
const device = instance.createLogicalDevice(physical);

// 2. Allocate memory
const mm = device.memoryManager;
const staging = mm.allocate(64, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT, BufferUsage.TRANSFER_SRC);
const deviceBuf = mm.allocate(64, MemoryType.DEVICE_LOCAL, BufferUsage.STORAGE | BufferUsage.TRANSFER_DST);

// 3. Upload data via staging buffer
const mapped = mm.map(staging);
mapped.write(0, new Uint8Array(64).fill(0x42));
mm.unmap(staging);

// 4. Create pipeline
const shader = device.createShaderModule({ code: [limm(0, 1.0), halt()], localSize: [32, 1, 1] });
const dsLayout = device.createDescriptorSetLayout([{ binding: 0, type: "storage" }]);
const plLayout = device.createPipelineLayout([dsLayout]);
const pipeline = device.createComputePipeline(shader, plLayout);

// 5. Record commands
const cb = device.createCommandBuffer();
cb.begin();
cb.cmdCopyBuffer(staging, deviceBuf, 64);
cb.cmdPipelineBarrier(makePipelineBarrier({
  srcStage: PipelineStage.TRANSFER,
  dstStage: PipelineStage.COMPUTE,
  memoryBarriers: [{ srcAccess: AccessFlags.TRANSFER_WRITE, dstAccess: AccessFlags.SHADER_READ }],
}));
cb.cmdBindPipeline(pipeline);
cb.cmdDispatch(1, 1, 1);
cb.end();

// 6. Submit and wait
const fence = device.createFence();
const queue = device.queues["compute"][0];
queue.submit([cb], { fence });
console.log(fence.signaled); // true
```

## Supported Device Types

| Vendor | Device Type | Memory Model |
|--------|------------|--------------|
| NVIDIA | GPU | Discrete (HBM + staging) |
| AMD | GPU | Discrete (GDDR6 + staging) |
| Google | TPU | Discrete (HBM) |
| Intel | GPU | Discrete (GDDR6) |
| Apple | ANE | Unified (zero-copy) |

## Memory Model

The runtime supports two memory patterns:

**Discrete GPUs** (NVIDIA, AMD, Intel, Google): Separate host and device memory. Data must be staged:
```
Host → staging buffer (HOST_VISIBLE) → copy command → device buffer (DEVICE_LOCAL)
```

**Unified Memory** (Apple): Single address space, zero-copy:
```
Host → unified buffer (DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT) → direct dispatch
```

## Command Buffer State Machine

```
INITIAL → begin() → RECORDING → end() → RECORDED → submit() → PENDING → [complete] → COMPLETE
    ↑                                                                                      |
    └──────────────────────────── reset() ←────────────────────────────────────────────────┘
```

## How it fits in the stack

This package sits at Layer 5, consuming `@coding-adventures/device-simulator` (Layer 6) which provides the actual hardware simulation. It adds the runtime/driver layer that real-world APIs like Vulkan, CUDA, Metal, and OpenCL implement.

## Dependencies

- `@coding-adventures/device-simulator` — Hardware device simulation (NVIDIA, AMD, Google, Intel, Apple)
- `@coding-adventures/gpu-core` — Instruction set for GPU-style shaders

## Testing

```bash
npm test              # Run tests
npm run test:coverage # Run tests with coverage report
```

181 tests, 96%+ line coverage across all modules.
