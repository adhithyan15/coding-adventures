# G05 — Compute Runtime

## Overview

This package implements **Layer 5 of the accelerator computing stack** — a
low-level compute runtime that provides the software infrastructure between
user-facing APIs (CUDA, OpenCL, Metal, Vulkan) and the hardware device
simulators (Layer 6).

Think of this as the **GPU driver's internal machinery**. When you call
`cudaMalloc()` or `vkAllocateMemory()`, those high-level calls eventually
reach a low-level runtime that manages physical devices, command submission,
memory allocation, and synchronization. That runtime is what we're building.

The design is inspired by **Vulkan** — the most explicit GPU API — because
it exposes all the moving parts that other APIs hide behind convenience
wrappers. If we get this layer right, building CUDA, OpenCL, Metal, and
Vulkan simulators on top (Layer 4) becomes a matter of adding convenience
and hiding complexity.

## Layer Position

```
Layer 11: Logic Gates (AND, OR, XOR, NAND)
    │
Layer 10: FP Arithmetic (IEEE 754 add/mul/fma)
    │
Layer  9: Accelerator Core (gpu-core) — one core, one instruction at a time
    │
Layer  8: Parallel Execution Engine — warps, wavefronts, systolic arrays
    │
Layer  7: Compute Unit — SM, CU, MXU, XeCore, ANECore
    │
Layer  6: Device Simulator — complete devices with global memory + work dist.
    │
Layer  5: Compute Runtime ← YOU ARE HERE
    │
    ├──→ Instance          — enumerate and select physical devices
    ├──→ LogicalDevice     — usable handle with command queues
    ├──→ CommandBuffer     — recorded sequence of GPU commands
    ├──→ CommandQueue      — FIFO submission with scheduling
    ├──→ MemoryManager     — allocation, mapping, memory types
    ├──→ Pipeline          — compiled kernel + execution state
    ├──→ DescriptorSet     — binds memory buffers to kernel parameters
    ├──→ Synchronization   — fences, semaphores, barriers, events
    └──→ RuntimeTrace      — submission-level observability
    │
Layer  4: API Simulators (future)
    │
    ├──→ CUDARuntime       — convenience wrapper (implicit sync, easy launch)
    ├──→ OpenCLRuntime      — cross-platform wrapper (context, programs, kernels)
    ├──→ MetalRuntime       — Apple-style wrapper (command encoders, MTLBuffer)
    └──→ VulkanRuntime      — thin wrapper (almost 1:1 with Layer 5)
```

**Depends on:**
- `device-simulator` (Layer 6) — AcceleratorDevice, KernelDescriptor
- `gpu-core` — Instruction, InstructionSet
- `clock` — cycle-driven simulation
- `fp-arithmetic` — shared FP operations

**Used by:** CUDA Runtime, OpenCL Runtime, Metal Runtime, Vulkan Runtime (Layer 4, future)

## Why Vulkan-Inspired?

There are four major GPU APIs, and they differ in how much they hide:

```
    More explicit                                      More implicit
    (you manage everything)                     (runtime manages for you)
    ◄─────────────────────────────────────────────────────────────────►

    Vulkan          Metal          OpenCL           CUDA
    ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
    │ Command  │   │ Command  │   │ Command  │   │ Implicit │
    │ buffers  │   │ encoders │   │ queues   │   │ stream   │
    │ Queues   │   │ Queues   │   │ Events   │   │ Default  │
    │ Fences   │   │ Fences   │   │ Barriers │   │ sync     │
    │ Barriers │   │ Barriers │   │          │   │          │
    │ Memory   │   │ Storage  │   │ Buffers  │   │ Unified  │
    │ types    │   │ modes    │   │ (auto)   │   │ alloc    │
    │ Pipelines│   │ Pipelines│   │ Programs │   │ Modules  │
    │ Desc sets│   │ Arguments│   │ Kernels  │   │ Functions│
    └──────────┘   └──────────┘   └──────────┘   └──────────┘
     Everything     Balanced       Moderate         Minimal
     explicit       control        control          control
```

We model at the Vulkan level because:
1. Every concept from CUDA/OpenCL/Metal has a Vulkan equivalent
2. CUDA can be built by adding defaults and implicit behavior
3. OpenCL can be built by adding cross-platform device selection
4. Metal can be built by mapping command encoders to command buffers
5. Vulkan itself is nearly 1:1 with our runtime

It's easier to add convenience than to remove hidden state.

## The Big Picture: How a GPU Program Actually Runs

When you write `y = model(x)` in PyTorch, here's what happens all the way down:

```
PyTorch                          "y = model(x)"
   │
CUDA Runtime (Layer 4)           cudaMalloc, cudaMemcpy, kernel<<<grid,block>>>
   │
Compute Runtime (Layer 5)  ←──  THIS LAYER
   │                             1. Find a GPU (Instance.enumerate_devices)
   │                             2. Create logical device + queue
   │                             3. Allocate buffers (MemoryManager)
   │                             4. Record commands (CommandBuffer)
   │                             5. Submit to queue (CommandQueue)
   │                             6. Wait for completion (Fence)
   │                             7. Read results back
   │
Device Simulator (Layer 6)       NvidiaGPU.launch_kernel, .step(), .run()
   │
Compute Units (Layer 7)          SM executes warps on actual compute cores
   │
   ... all the way down to logic gates
```

The runtime is the **command & control layer**. The device does the actual
computation. The runtime tells the device **what** to do and **when**.

## Core Concepts

### 1. Instance — The Entry Point

The Instance is how you discover what hardware is available. In Vulkan,
`vkEnumeratePhysicalDevices` returns a list of GPUs. Our Instance does the
same but across all device types.

```
Instance
├── enumerate_devices() → list[PhysicalDevice]
│   "What GPUs/TPUs/NPUs are plugged in?"
│
├── create_device(physical, queues) → LogicalDevice
│   "I want to use this GPU. Give me a handle."
│
└── properties() → InstanceProperties
    "What runtime version is this? What extensions?"
```

A **PhysicalDevice** is a read-only description of hardware. It tells you
the device name, type, memory sizes, and capabilities — but you can't use
it directly. You need to create a LogicalDevice first.

Why the separation? Real systems may have multiple GPUs. You query all of
them, pick the best one, and create a logical handle for it. The physical
device never changes. The logical device holds your allocated resources.

### 2. LogicalDevice — Your Handle to the Hardware

A LogicalDevice wraps a single PhysicalDevice and owns:
- One or more CommandQueues (for submitting work)
- A MemoryManager (for allocating device memory)
- The ability to create pipelines, command buffers, and sync objects

```
LogicalDevice
├── queues: list[CommandQueue]
│   "Submit work here"
│
├── memory_manager: MemoryManager
│   "Allocate device memory here"
│
├── create_command_buffer() → CommandBuffer
│   "Record GPU commands into this"
│
├── create_pipeline(shader, layout) → Pipeline
│   "Compile this shader for execution"
│
├── create_fence() → Fence
│   "CPU waits on this until GPU signals it"
│
├── create_semaphore() → Semaphore
│   "GPU-to-GPU synchronization"
│
├── create_event() → Event
│   "Fine-grained GPU-side signaling"
│
└── wait_idle()
    "Block until ALL work on this device is done"
```

### 3. CommandBuffer — A Recorded List of GPU Commands

This is the key Vulkan concept that makes everything explicit. Instead of
calling GPU operations one at a time (like CUDA does), you **record** a
sequence of commands into a buffer, then **submit** the whole buffer at
once.

Why record-then-submit?
1. **Batch optimization** — the driver can see all commands at once and
   optimize the sequence (reorder, merge, eliminate redundancies)
2. **Reuse** — you can submit the same command buffer multiple times
   without re-recording (e.g., same inference pass on different inputs)
3. **Multi-threaded recording** — different CPU threads can record
   different command buffers in parallel, then submit them together
4. **Validation** — the runtime can validate the entire sequence before
   any GPU work starts, catching errors early

```
CommandBuffer lifecycle:

    ┌─────────┐     begin()     ┌────────────┐    end()     ┌──────────┐
    │ Initial │ ──────────────► │ Recording  │ ───────────► │ Recorded │
    └─────────┘                 └────────────┘              └────┬─────┘
                                     │                          │
                                     │ cmd_bind_pipeline()      │ submit()
                                     │ cmd_bind_descriptor()    │
                                     │ cmd_dispatch()           ▼
                                     │ cmd_copy_buffer()   ┌──────────┐
                                     │ cmd_barrier()       │ Pending  │
                                     │                     └────┬─────┘
                                     │                          │
                                     │                          │ GPU completes
                                     │                          ▼
                                     │                     ┌──────────┐
                                     │                     │ Complete │
                                     │                     └──────────┘
                                     │                          │
                                     │                          │ reset()
                                     ▼                          │
                                ┌───────────┐                   │
                                │ Reuse: ◄──────────────────────┘
                                │ begin()   │
                                └───────────┘
```

Commands you can record:

| Command | What it does |
|---------|-------------|
| `cmd_bind_pipeline` | Select which compiled kernel to run |
| `cmd_bind_descriptor_set` | Bind memory buffers to kernel parameters |
| `cmd_dispatch(gx, gy, gz)` | Launch a compute kernel with grid dimensions |
| `cmd_dispatch_indirect(buf)` | Read grid dimensions from a GPU buffer |
| `cmd_copy_buffer(src, dst, size)` | Device-to-device memory copy |
| `cmd_fill_buffer(buf, value)` | Fill buffer with a constant |
| `cmd_update_buffer(buf, data)` | Write small amounts of data inline |
| `cmd_pipeline_barrier(...)` | Insert a memory/execution barrier |
| `cmd_reset_event(event)` | Reset an event from GPU side |
| `cmd_set_event(event)` | Signal an event from GPU side |
| `cmd_wait_event(event)` | Wait for event before proceeding |
| `cmd_push_constants(data)` | Small inline data for the kernel (≤128 bytes) |

### 4. CommandQueue — Where Recorded Commands Get Submitted

A queue is a FIFO that accepts command buffers and feeds them to the device.
Real GPUs have multiple queue types:

```
┌─────────────────────────────────────────────┐
│                 LogicalDevice                 │
│                                               │
│   ┌───────────────┐  ┌───────────────┐       │
│   │ Compute Queue │  │ Transfer Queue│       │
│   │               │  │               │       │
│   │ [CB 3]        │  │ [CB 5]        │       │
│   │ [CB 2]        │  │               │       │
│   │ [CB 1] ◄──    │  │ [CB 4] ◄──   │       │
│   └───────┬───────┘  └───────┬───────┘       │
│           │                  │                │
│           ▼                  ▼                │
│   ┌────────────────────────────────────────┐ │
│   │           Device Simulator              │ │
│   │  (SMs execute compute, DMA does copy)   │ │
│   └────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

Queue types:
- **Compute** — kernel execution (dispatch commands)
- **Transfer** — memory copy operations (DMA)
- **Compute + Transfer** — can do both (most common)

Why multiple queues? Overlap. While the GPU is running a kernel on the
compute queue, the DMA engine can be copying data for the next kernel on
the transfer queue. This is how you hide PCIe transfer latency.

### 5. MemoryManager — Typed Allocations

GPU memory isn't one flat pool. Different memory locations have different
properties:

```
Memory Types:

    ┌─────────────────────┐
    │   DEVICE_LOCAL       │  ← fastest for GPU, invisible to CPU
    │   (VRAM / HBM)      │    Use: textures, weight buffers, intermediates
    │   Speed: ████████    │
    └─────────────────────┘

    ┌─────────────────────┐
    │   HOST_VISIBLE +     │  ← CPU can write, GPU can read
    │   HOST_COHERENT      │    Use: staging buffers for uploads
    │   Speed: ████        │    (CPU writes are instantly visible to GPU)
    └─────────────────────┘

    ┌─────────────────────┐
    │   HOST_VISIBLE +     │  ← CPU can write, must flush for GPU to see
    │   HOST_CACHED        │    Use: read-back buffers
    │   Speed: █████       │    (faster CPU reads, requires explicit flush)
    └─────────────────────┘

    ┌─────────────────────┐
    │   DEVICE_LOCAL +     │  ← Apple unified / resizable BAR
    │   HOST_VISIBLE       │    Use: zero-copy on unified memory
    │   Speed: ████████    │    (GPU speed, CPU accessible)
    └─────────────────────┘
```

The pattern for discrete GPUs (NVIDIA, AMD, Intel):
1. Allocate a HOST_VISIBLE staging buffer
2. Write data from CPU into the staging buffer
3. Record a `cmd_copy_buffer` to copy staging → DEVICE_LOCAL
4. Submit and wait
5. Now the GPU has the data at full speed

The pattern for unified memory (Apple):
1. Allocate a DEVICE_LOCAL + HOST_VISIBLE buffer
2. Write data directly — both CPU and GPU see it immediately
3. No copy needed

### 6. Pipeline — A Compiled Kernel Ready to Execute

A Pipeline packages everything needed to run a kernel:
- The compiled shader/program
- The descriptor set layout (what buffers it expects)
- Push constant layout (small inline parameters)

```
Pipeline creation:

    ShaderModule              DescriptorSetLayout
    (compiled code)           (what buffers the kernel reads/writes)
         │                           │
         ▼                           ▼
    ┌────────────────────────────────────────┐
    │             PipelineLayout              │
    │  "This kernel takes 3 buffers and      │
    │   16 bytes of push constants"           │
    └───────────────────┬────────────────────┘
                        │
                        ▼
    ┌────────────────────────────────────────┐
    │              Pipeline                   │
    │  "Compiled and ready to dispatch"       │
    │                                         │
    │  Bound via: cmd_bind_pipeline()         │
    └────────────────────────────────────────┘
```

A **ShaderModule** wraps a compiled program. For GPU-style devices, this
is a list of instructions (our existing `Instruction` type from gpu-core).
For dataflow devices, this is an operation descriptor. The shader module
is device-agnostic — the pipeline compilation step adapts it to the
specific device.

A **DescriptorSetLayout** describes the shape of data bindings:
- Binding 0: Storage buffer (read-only) — input data
- Binding 1: Storage buffer (read-only) — weights
- Binding 2: Storage buffer (read-write) — output

A **DescriptorSet** is a concrete instance of a layout with actual buffers
bound to each slot. You can create multiple descriptor sets from the same
layout, each pointing to different data.

### 7. Synchronization — Coordinating CPU and GPU

The CPU and GPU run asynchronously. When you submit a command buffer, the
CPU doesn't wait — it keeps going. You need explicit synchronization to
know when the GPU is done.

```
    CPU timeline:  ───[submit CB]──────[submit CB]──[wait fence]──[read data]──►
                         │                  │            ▲
                         ▼                  ▼            │
    GPU timeline:  ──────[execute CB 1]─────[execute CB 2]──[signal fence]──────►
                                                              │
                                                    Fence fires when
                                                    GPU finishes CB 2
```

Three synchronization primitives:

**Fence** — CPU ↔ GPU synchronization
- CPU submits work with a fence attached
- CPU calls `fence.wait()` to block until GPU signals it
- Use case: "Wait until my kernel is done so I can read results"

**Semaphore** — GPU ↔ GPU synchronization (between queues)
- Queue A signals a semaphore when its command buffer completes
- Queue B waits on that semaphore before starting
- Use case: "Don't start the compute kernel until the data transfer finishes"

**Event** — fine-grained GPU-side signaling
- Recorded inside command buffers with `cmd_set_event` / `cmd_wait_event`
- Use case: "Barrier between two dispatches within the same command buffer"

**Pipeline Barrier** — execution + memory ordering within a command buffer
- Ensures that all commands before the barrier complete before commands after
- Also handles memory visibility (flush caches so writes are visible to reads)
- Use case: "Kernel A writes to buffer X, kernel B reads from buffer X"

```
Pipeline Barrier anatomy:

    cmd_dispatch(kernel_A)
         │
         │  Writes to buffer X go to L2 cache
         │
    cmd_pipeline_barrier(
        src_stage = COMPUTE,      ← "wait for compute to finish"
        dst_stage = COMPUTE,      ← "before starting next compute"
        memory_barrier = {
            src_access = WRITE,   ← "flush writes"
            dst_access = READ,    ← "invalidate read caches"
        }
    )
         │
         │  Buffer X is now visible for reading
         │
    cmd_dispatch(kernel_B)
```

### 8. RuntimeTrace — Submission-Level Observability

While DeviceTrace (Layer 6) shows cycle-by-cycle hardware activity,
RuntimeTrace shows the software-level view: what was submitted, when,
and how long it took.

```
RuntimeTrace:
    [T=0ms]   Submit CB#1 to compute queue (fence=F1)
    [T=0ms]   Submit CB#2 to transfer queue (semaphore=S1)
    [T=2ms]   Semaphore S1 signaled — transfer complete
    [T=2ms]   CB#3 begins (was waiting on S1)
    [T=15ms]  Fence F1 signaled — compute kernel done
    [T=15ms]  CPU reads back results
```

## Protocol Definitions

### RuntimeInstance

```
RuntimeInstance
├── enumerate_physical_devices() → list[PhysicalDevice]
├── create_logical_device(physical, queue_requests) → LogicalDevice
└── version: str
```

### PhysicalDevice

```
PhysicalDevice (read-only view of hardware)
├── device_id: int
├── name: str                              "NVIDIA H100"
├── device_type: DeviceType                GPU, TPU, NPU
├── vendor: str                            "nvidia", "amd", "google", etc.
├── memory_properties: MemoryProperties    types, heaps, sizes
├── queue_families: list[QueueFamily]      compute, transfer, etc.
├── limits: DeviceLimits                   max workgroup size, buffer size, etc.
└── supports_feature(feature) → bool       "does this GPU support FP16?"
```

### LogicalDevice

```
LogicalDevice
├── physical_device: PhysicalDevice
├── queues: dict[QueueType, list[CommandQueue]]
├── memory_manager: MemoryManager
│
├── create_command_buffer(queue_type) → CommandBuffer
├── create_shader_module(code) → ShaderModule
├── create_descriptor_set_layout(bindings) → DescriptorSetLayout
├── create_pipeline_layout(set_layouts, push_constant_size) → PipelineLayout
├── create_compute_pipeline(shader, layout) → Pipeline
├── create_descriptor_set(layout) → DescriptorSet
│
├── create_fence(signaled=False) → Fence
├── create_semaphore() → Semaphore
├── create_event() → Event
│
├── wait_idle() → None        block until all queues drain
├── reset() → None            reset all state
└── stats: RuntimeStats       aggregate submission statistics
```

### MemoryManager

```
MemoryManager
├── memory_properties: MemoryProperties
│
├── allocate(size, memory_type) → Buffer
│   Allocate typed device memory.
│   memory_type: DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT, etc.
│
├── free(buffer) → None
│
├── map(buffer) → MappedMemory
│   Map device buffer into CPU-accessible address space.
│   Only valid for HOST_VISIBLE buffers.
│   Returns a handle for reading/writing bytes from CPU side.
│
├── unmap(buffer) → None
│   Release CPU mapping.
│
├── flush(buffer, offset, size) → None
│   Flush CPU writes to make them visible to GPU.
│   Only needed for HOST_VISIBLE without HOST_COHERENT.
│
├── invalidate(buffer, offset, size) → None
│   Invalidate CPU cache so GPU writes become visible.
│   Only needed for read-back buffers.
│
└── stats: MemoryStats
    Total allocated, peak usage, allocation count, etc.
```

### Buffer

```
Buffer
├── id: int
├── size: int
├── memory_type: MemoryType
├── device_address: int         address on the device (from Layer 6 malloc)
├── mapped: bool                is this buffer currently CPU-mapped?
└── usage: BufferUsage          STORAGE, UNIFORM, TRANSFER_SRC, TRANSFER_DST
```

### CommandBuffer

```
CommandBuffer
├── state: CommandBufferState       INITIAL, RECORDING, RECORDED, PENDING, COMPLETE
│
├── begin() → None                  transition to RECORDING
├── end() → None                    transition to RECORDED
├── reset() → None                  transition back to INITIAL
│
│  === Compute commands ===
├── cmd_bind_pipeline(pipeline) → None
├── cmd_bind_descriptor_set(set) → None
├── cmd_push_constants(offset, data) → None
├── cmd_dispatch(group_x, group_y, group_z) → None
├── cmd_dispatch_indirect(buffer, offset) → None
│
│  === Transfer commands ===
├── cmd_copy_buffer(src, dst, size, src_offset=0, dst_offset=0) → None
├── cmd_fill_buffer(buffer, value, offset=0, size=WHOLE) → None
├── cmd_update_buffer(buffer, offset, data) → None
│
│  === Synchronization commands ===
├── cmd_pipeline_barrier(barrier) → None
├── cmd_set_event(event, stage) → None
├── cmd_wait_event(event, src_stage, dst_stage) → None
├── cmd_reset_event(event, stage) → None
│
└── commands: list[RecordedCommand]   all recorded commands (for inspection)
```

### CommandQueue

```
CommandQueue
├── queue_type: QueueType           COMPUTE, TRANSFER, COMPUTE_TRANSFER
├── queue_index: int                which queue of this type (0, 1, ...)
│
├── submit(command_buffers, wait_semaphores, signal_semaphores, fence) → None
│   Submit one or more command buffers for execution.
│   - wait_semaphores: don't start until these are signaled
│   - signal_semaphores: signal these when all CBs complete
│   - fence: signal this fence when all CBs complete (for CPU waiting)
│
├── wait_idle() → None              block until this queue drains
│
├── pending_count: int              command buffers waiting to execute
└── executing: bool                 is a command buffer currently running?
```

### Pipeline

```
Pipeline
├── shader: ShaderModule
├── layout: PipelineLayout
├── pipeline_id: int
└── workgroup_size: tuple[int, int, int]   local workgroup dimensions
```

### ShaderModule

```
ShaderModule
├── module_id: int
├── code: list[Instruction] | OperationDescriptor
│   GPU: list of ISA instructions
│   Dataflow: operation name + parameters
├── entry_point: str                  "main" by default
└── local_size: tuple[int, int, int]  workgroup dimensions declared in shader
```

### DescriptorSetLayout & DescriptorSet

```
DescriptorSetLayout
├── bindings: list[DescriptorBinding]
│   Each binding: slot number, type (STORAGE/UNIFORM), count
└── layout_id: int

DescriptorSet
├── layout: DescriptorSetLayout
├── set_id: int
├── write(binding, buffer) → None
│   Bind a concrete buffer to a slot.
└── bindings: dict[int, Buffer]
```

### Synchronization Primitives

```
Fence (CPU ↔ GPU)
├── signaled: bool
├── wait(timeout_cycles=None) → bool    block CPU until signaled
├── reset() → None                      clear signal for reuse
└── fence_id: int

Semaphore (GPU ↔ GPU)
├── signaled: bool
├── semaphore_id: int
└── (no CPU-side wait — only used in queue submit)

Event (GPU ↔ GPU, fine-grained)
├── signaled: bool
├── event_id: int
├── set() → None                        signal from CPU
├── reset() → None                      reset from CPU
└── status() → bool                     check without blocking
```

### Barrier

```
PipelineBarrier
├── src_stage: PipelineStage      COMPUTE, TRANSFER, TOP, BOTTOM
├── dst_stage: PipelineStage
├── memory_barriers: list[MemoryBarrier]
│
│   MemoryBarrier:
│   ├── src_access: AccessFlags   WRITE, READ, TRANSFER_WRITE, ...
│   └── dst_access: AccessFlags
│
└── buffer_barriers: list[BufferBarrier]
    BufferBarrier:
    ├── buffer: Buffer
    ├── src_access: AccessFlags
    ├── dst_access: AccessFlags
    ├── offset: int
    └── size: int
```

## Enumerations

```
DeviceType:       GPU, TPU, NPU

QueueType:        COMPUTE, TRANSFER, COMPUTE_TRANSFER

MemoryType (flags, combinable):
    DEVICE_LOCAL          GPU-fast memory (VRAM/HBM)
    HOST_VISIBLE          CPU can read/write
    HOST_COHERENT         CPU writes immediately visible to GPU
    HOST_CACHED           CPU reads are cached (fast read-back)

BufferUsage (flags, combinable):
    STORAGE               Shader can read/write (SSBO in Vulkan)
    UNIFORM               Shader can read (small, fast, UBO)
    TRANSFER_SRC          Can be source of a copy
    TRANSFER_DST          Can be destination of a copy
    INDIRECT              Contains indirect dispatch parameters

PipelineStage:
    TOP_OF_PIPE           Before any work
    COMPUTE               Compute shader execution
    TRANSFER              Copy/fill/update operations
    HOST                  CPU access
    BOTTOM_OF_PIPE        After all work

AccessFlags:
    SHADER_READ           Compute shader read
    SHADER_WRITE          Compute shader write
    TRANSFER_READ         Copy source
    TRANSFER_WRITE        Copy destination
    HOST_READ             CPU read (after map)
    HOST_WRITE            CPU write (after map)

CommandBufferState:
    INITIAL               Just created or reset
    RECORDING             Between begin() and end()
    RECORDED              Commands recorded, ready to submit
    PENDING               Submitted, waiting for or being executed by GPU
    COMPLETE              GPU finished executing all commands
```

## End-to-End Example: SAXPY on NVIDIA GPU

Here's the complete flow for Y = alpha * X + Y on an NVIDIA GPU,
showing every runtime call:

```python
from compute_runtime import (
    RuntimeInstance, MemoryType, BufferUsage,
    PipelineStage, AccessFlags,
)
from gpu_core import limm, load, fmul, fadd, store, halt

# ── Step 1: Discover and select a device ──────────────────────────

instance = RuntimeInstance()
devices = instance.enumerate_physical_devices()

# Pick the NVIDIA GPU
nvidia = next(d for d in devices if d.vendor == "nvidia")
print(nvidia.name)              # "NVIDIA H100"
print(nvidia.memory_properties) # 80 GB DEVICE_LOCAL, staging pools

# Create a logical device with one compute queue
device = instance.create_logical_device(
    physical_device=nvidia,
    queue_requests=[{"type": "compute", "count": 1}],
)
compute_queue = device.queues["compute"][0]

# ── Step 2: Allocate memory ───────────────────────────────────────

N = 1024  # vector length
mm = device.memory_manager

# Device-local buffers (fast, GPU-only)
buf_x = mm.allocate(N * 4, MemoryType.DEVICE_LOCAL,
                    usage=BufferUsage.STORAGE | BufferUsage.TRANSFER_DST)
buf_y = mm.allocate(N * 4, MemoryType.DEVICE_LOCAL,
                    usage=BufferUsage.STORAGE | BufferUsage.TRANSFER_DST)

# Staging buffer (CPU-writable, used to upload data)
staging = mm.allocate(N * 4 * 2, MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
                      usage=BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST)

# ── Step 3: Upload data via staging buffer ────────────────────────

mapped = mm.map(staging)
mapped.write(0, x_data_bytes)          # Write X at offset 0
mapped.write(N * 4, y_data_bytes)      # Write Y at offset N*4
mm.unmap(staging)

# Record transfer commands
upload_cb = device.create_command_buffer("compute")
upload_cb.begin()
upload_cb.cmd_copy_buffer(staging, buf_x, N * 4, src_offset=0)
upload_cb.cmd_copy_buffer(staging, buf_y, N * 4, src_offset=N * 4)
upload_cb.cmd_pipeline_barrier(
    src_stage=PipelineStage.TRANSFER,
    dst_stage=PipelineStage.COMPUTE,
    src_access=AccessFlags.TRANSFER_WRITE,
    dst_access=AccessFlags.SHADER_READ,
)
upload_cb.end()

# ── Step 4: Create the SAXPY pipeline ─────────────────────────────

#   r0 = alpha (push constant)
#   r1 = X[tid] (load from binding 0)
#   r2 = Y[tid] (load from binding 1)
#   r3 = alpha * X[tid]
#   r4 = alpha * X[tid] + Y[tid]
#   store r4 → Y[tid] (binding 1)
saxpy_code = [
    limm(0, 2.0),          # r0 = alpha = 2.0
    load(1, 8, 0),         # r1 = X[tid]  (binding 0 base in r8)
    load(2, 9, 0),         # r2 = Y[tid]  (binding 1 base in r9)
    fmul(3, 0, 1),         # r3 = alpha * X[tid]
    fadd(4, 3, 2),         # r4 = r3 + Y[tid]
    store(4, 9, 0),        # Y[tid] = r4
    halt(),
]

shader = device.create_shader_module(
    code=saxpy_code,
    local_size=(256, 1, 1),
)

# Descriptor set layout: 2 storage buffers
ds_layout = device.create_descriptor_set_layout([
    {"binding": 0, "type": "storage", "count": 1},   # X
    {"binding": 1, "type": "storage", "count": 1},   # Y
])

pipeline_layout = device.create_pipeline_layout(
    set_layouts=[ds_layout],
    push_constant_size=4,  # alpha (one float)
)

pipeline = device.create_compute_pipeline(shader, pipeline_layout)

# Bind actual buffers to descriptor set
desc_set = device.create_descriptor_set(ds_layout)
desc_set.write(0, buf_x)
desc_set.write(1, buf_y)

# ── Step 5: Record compute dispatch ───────────────────────────────

compute_cb = device.create_command_buffer("compute")
compute_cb.begin()
compute_cb.cmd_bind_pipeline(pipeline)
compute_cb.cmd_bind_descriptor_set(desc_set)
compute_cb.cmd_push_constants(0, struct.pack("f", 2.0))  # alpha
compute_cb.cmd_dispatch(N // 256, 1, 1)   # 4 workgroups of 256 threads
compute_cb.cmd_pipeline_barrier(
    src_stage=PipelineStage.COMPUTE,
    dst_stage=PipelineStage.TRANSFER,
    src_access=AccessFlags.SHADER_WRITE,
    dst_access=AccessFlags.TRANSFER_READ,
)
compute_cb.end()

# ── Step 6: Record download ───────────────────────────────────────

download_cb = device.create_command_buffer("compute")
download_cb.begin()
download_cb.cmd_copy_buffer(buf_y, staging, N * 4)
download_cb.end()

# ── Step 7: Submit all three command buffers ──────────────────────

fence = device.create_fence()

# Submit upload → compute → download with a fence at the end
compute_queue.submit(
    command_buffers=[upload_cb, compute_cb, download_cb],
    fence=fence,
)

# CPU blocks here until GPU finishes everything
fence.wait()

# ── Step 8: Read results ──────────────────────────────────────────

mapped = mm.map(staging)
result_bytes = mapped.read(0, N * 4)
mm.unmap(staging)
# result_bytes now contains Y = 2.0 * X + Y

# ── Cleanup ───────────────────────────────────────────────────────

mm.free(buf_x)
mm.free(buf_y)
mm.free(staging)
```

### Same Example on Apple ANE (Unified Memory)

Notice how much simpler unified memory makes things — no staging
buffer, no upload/download commands:

```python
instance = RuntimeInstance()
devices = instance.enumerate_physical_devices()
apple = next(d for d in devices if d.vendor == "apple")

device = instance.create_logical_device(
    physical_device=apple,
    queue_requests=[{"type": "compute", "count": 1}],
)

mm = device.memory_manager

# Unified memory: DEVICE_LOCAL + HOST_VISIBLE — both CPU and GPU see it
buf_x = mm.allocate(N * 4,
    MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE,
    usage=BufferUsage.STORAGE)
buf_y = mm.allocate(N * 4,
    MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE,
    usage=BufferUsage.STORAGE)

# Write directly — no staging buffer needed!
mapped_x = mm.map(buf_x)
mapped_x.write(0, x_data_bytes)
mm.unmap(buf_x)

mapped_y = mm.map(buf_y)
mapped_y.write(0, y_data_bytes)
mm.unmap(buf_y)

# Record compute (no upload/download commands needed)
cb = device.create_command_buffer("compute")
cb.begin()
cb.cmd_bind_pipeline(pipeline)   # same pipeline as before
cb.cmd_bind_descriptor_set(desc_set)
cb.cmd_dispatch(N // 256, 1, 1)
cb.end()

fence = device.create_fence()
device.queues["compute"][0].submit([cb], fence=fence)
fence.wait()

# Read directly — no download needed!
mapped_y = mm.map(buf_y)
result_bytes = mapped_y.read(0, N * 4)
mm.unmap(buf_y)
```

Two command buffers instead of three. Zero copy overhead. This is exactly
why Apple's unified memory architecture matters — it eliminates an entire
class of complexity.

## How the Runtime Translates to Layer 6

The runtime doesn't simulate hardware — it **drives** the hardware
simulator. Here's how each runtime operation maps to Layer 6 calls:

```
Runtime operation              Layer 6 (Device Simulator) call
─────────────────              ─────────────────────────────────
mm.allocate(size, type)   →    device.malloc(size)
mm.free(buffer)           →    device.free(buffer.device_address)
mm.map(buffer)            →    return MappedMemory pointing to device memory
mm.unmap(buffer)          →    (no-op if coherent, flush if not)

cmd_copy_buffer(s, d, n)  →    device.memcpy_device_to_device(s, d, n)
                               (or memcpy_host_to_device for staging)

cmd_dispatch(gx, gy, gz)  →    device.launch_kernel(KernelDescriptor(
                                   program=pipeline.shader.code,
                                   grid_dim=(gx, gy, gz),
                                   block_dim=pipeline.shader.local_size,
                               ))

queue.submit(cbs, fence)  →    For each command in each CB:
                                   Execute the mapped Layer 6 operation
                               device.run(max_cycles)
                               fence.signal()

fence.wait()              →    Block until the device completes
                               (return accumulated DeviceTraces)
```

The runtime adds **ordering**, **synchronization**, and **validation**
on top of raw device operations. The device doesn't know about command
buffers or fences — it just executes kernels and manages memory. The
runtime sequences those operations correctly.

## Execution Model

### Command Buffer Execution

When a command buffer is submitted, the runtime processes commands
sequentially. Each command either:
- Maps to a Layer 6 device operation (dispatch, copy, fill)
- Modifies runtime state (bind pipeline, bind descriptors)
- Inserts ordering constraints (barriers, events)

```
CommandBuffer execution loop:

    for cmd in command_buffer.commands:
        match cmd:
            case BindPipeline(p):
                current_pipeline = p

            case BindDescriptorSet(ds):
                current_descriptors = ds

            case Dispatch(gx, gy, gz):
                kernel = build_kernel_descriptor(
                    current_pipeline, current_descriptors, gx, gy, gz
                )
                device.launch_kernel(kernel)
                traces += device.run(max_cycles)

            case CopyBuffer(src, dst, size):
                data, _ = device.memcpy_device_to_host(src.address, size)
                device.memcpy_host_to_device(dst.address, data)

            case PipelineBarrier(barrier):
                # In simulation: ensure all prior operations complete
                # (already handled by run() completing)
                # Track for validation and trace output
                record_barrier(barrier)

            case SetEvent(event):
                event.signal()

            case WaitEvent(event):
                assert event.signaled, "deadlock: waiting on unsignaled event"
```

### Queue Scheduling

When multiple command buffers are submitted to a queue, they execute
in FIFO order. When multiple queues exist, they execute concurrently
(limited by the device's actual parallelism).

```
Queue scheduling:

    Compute Queue:   [CB1] → [CB2] → [CB3]     (sequential within queue)
    Transfer Queue:  [CB4] → [CB5]              (sequential within queue)
                                                 (parallel between queues)

    Semaphore S1 links them:
    Transfer Queue signals S1 after CB4
    Compute Queue waits on S1 before CB2

    Timeline:
    Transfer: ══[CB4]══╗    ═══[CB5]═══
                        ║S1
    Compute:  ═[CB1]════╬═══[CB2]════[CB3]═══
                        ║
                   CB2 starts only after CB4 finishes
```

## RuntimeTrace

The runtime produces traces at a different granularity than device traces.
Device traces are per-cycle. Runtime traces are per-submission.

```python
@dataclass
class RuntimeTrace:
    """One runtime-level event."""
    timestamp_cycles: int               # when this happened
    event_type: RuntimeEventType        # SUBMIT, COMPLETE, FENCE, SEMAPHORE, ...
    description: str                    # human-readable
    queue_type: QueueType | None        # which queue
    command_buffer_id: int | None       # which CB
    fence_id: int | None                # which fence
    semaphore_id: int | None            # which semaphore
    device_traces: list[DeviceTrace]    # underlying device traces (if any)


class RuntimeEventType(Enum):
    SUBMIT = "submit"                   # CB submitted to queue
    BEGIN_EXECUTION = "begin_execution" # CB starts running on device
    END_EXECUTION = "end_execution"     # CB finishes on device
    FENCE_SIGNAL = "fence_signal"       # Fence signaled
    FENCE_WAIT = "fence_wait"           # CPU waiting on fence
    SEMAPHORE_SIGNAL = "semaphore_signal"
    SEMAPHORE_WAIT = "semaphore_wait"
    BARRIER = "barrier"                 # Pipeline barrier executed
    MEMORY_ALLOC = "memory_alloc"       # Buffer allocated
    MEMORY_FREE = "memory_free"         # Buffer freed
    MEMORY_MAP = "memory_map"           # Buffer mapped to CPU
    MEMORY_TRANSFER = "memory_transfer" # Host ↔ device copy
```

## RuntimeStats

```python
@dataclass
class RuntimeStats:
    """Aggregate statistics for the runtime."""
    # Submissions
    total_submissions: int = 0
    total_command_buffers: int = 0
    total_dispatches: int = 0
    total_transfers: int = 0
    total_barriers: int = 0

    # Synchronization
    total_fence_waits: int = 0
    total_semaphore_signals: int = 0
    total_fence_wait_cycles: int = 0     # time CPU spent waiting

    # Memory
    total_allocated_bytes: int = 0
    peak_allocated_bytes: int = 0
    total_allocations: int = 0
    total_frees: int = 0
    total_maps: int = 0

    # Timing
    total_device_cycles: int = 0         # cycles the device was busy
    total_idle_cycles: int = 0           # cycles between submissions
    gpu_utilization: float = 0.0         # busy / (busy + idle)

    # Traces
    traces: list[RuntimeTrace] = field(default_factory=list)
```

## Validation

The runtime validates commands to catch errors early (before GPU execution):

1. **State machine validation**
   - Can't record commands unless CB is in RECORDING state
   - Can't submit unless CB is in RECORDED state
   - Can't dispatch without binding a pipeline first
   - Can't dispatch without binding a descriptor set first

2. **Memory validation**
   - Can't map a DEVICE_LOCAL-only buffer
   - Can't copy to a buffer without TRANSFER_DST usage
   - Can't bind a buffer to a STORAGE slot if it lacks STORAGE usage
   - Can't read from a buffer after writing without a barrier

3. **Synchronization validation**
   - Warn if dispatching without a barrier after a write to the same buffer
   - Warn if reading back results without waiting on a fence
   - Error if waiting on an event that can never be signaled (deadlock)

4. **Resource validation**
   - Can't use freed buffers
   - Can't exceed device memory limits
   - Can't exceed max workgroup size

## Package Structure

```
compute-runtime/
├── src/
│   ├── protocols.py              # All enums, dataclasses, Protocol types
│   ├── instance.py               # RuntimeInstance, PhysicalDevice
│   ├── device.py                 # LogicalDevice
│   ├── memory.py                 # MemoryManager, Buffer, MappedMemory
│   ├── command_buffer.py         # CommandBuffer with all cmd_* methods
│   ├── command_queue.py          # CommandQueue, submission logic
│   ├── pipeline.py               # ShaderModule, Pipeline, DescriptorSet
│   ├── sync.py                   # Fence, Semaphore, Event, Barrier
│   ├── trace.py                  # RuntimeTrace, RuntimeStats
│   └── validation.py             # ValidationLayer (optional, like Vulkan)
└── tests/
    ├── test_instance.py          # Device enumeration, creation
    ├── test_memory.py            # Allocate, map, free, memory types
    ├── test_command_buffer.py    # Record, state transitions, validation
    ├── test_command_queue.py     # Submit, FIFO ordering, fences
    ├── test_pipeline.py          # Shader compile, descriptor binding
    ├── test_sync.py              # Fences, semaphores, events, barriers
    ├── test_validation.py        # Error cases, misuse detection
    ├── test_execution.py         # End-to-end kernel dispatch
    └── test_multi_queue.py       # Overlapping compute and transfer
```

## Implementation Order

1. Write spec (this document) — commit
2. **Python** compute-runtime:
   a. `protocols.py` — all enums, dataclasses, Protocol types
   b. `instance.py` — device enumeration and logical device creation
   c. `memory.py` — MemoryManager, Buffer, MappedMemory, memory types
   d. `command_buffer.py` — recording, state machine, all cmd_* methods
   e. `pipeline.py` — ShaderModule, Pipeline, DescriptorSet
   f. `sync.py` — Fence, Semaphore, Event, PipelineBarrier
   g. `command_queue.py` — submit, execution loop, queue scheduling
   h. `trace.py` — RuntimeTrace, RuntimeStats
   i. `validation.py` — ValidationLayer
   j. Tests for each module
3. **TypeScript** — port
4. **Rust** — port
5. **Go** — port
6. **Ruby** — port
7. READMEs, CHANGELOGs, BUILD files (for ALL languages!)
8. PR

## Verification

Per language:
- All tests pass
- Coverage 90%+
- Linters pass
- BUILD file exists
- SAXPY end-to-end: allocate → upload → dispatch → download → verify
- Multi-queue: overlap transfer and compute via semaphores
- Fence wait: CPU blocks until GPU completes
- Memory types: staging upload pattern works on discrete GPU
- Unified memory: zero-copy pattern works on Apple ANE
- Validation: catches use-before-bind, missing barrier, freed buffer
- Command buffer reuse: record once, submit twice with different data

## Dependencies

- **Consumes:** `device-simulator` (Layer 6), `gpu-core`, `clock`, `fp-arithmetic`
- **Consumed by (future):** CUDA Runtime, OpenCL Runtime, Metal Runtime, Vulkan Runtime (Layer 4)
