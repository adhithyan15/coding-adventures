# G06 — Vendor API Simulators

## Overview

This package implements **Layer 3 of the accelerator computing stack** — a
collection of six vendor API simulators that provide the programming interfaces
developers actually use to program GPUs and accelerators. Each simulator is a
thin wrapper over the Vulkan-inspired compute runtime (Layer 4), translating
vendor-specific API calls into the common low-level operations underneath.

Think of it this way: Layer 4 is the **GPU driver internals**. Layer 3 is the
**API the programmer sees**. When a CUDA developer writes `cudaMalloc()`, the
CUDA runtime translates that into the explicit memory allocation, memory type
selection, and buffer creation that our Layer 4 handles. When a WebGPU developer
calls `device.createBuffer()`, the browser's WebGPU implementation does the same
translation internally.

The six simulators we build here represent the major GPU programming paradigms:

| Simulator | Paradigm | Real-world Users |
|-----------|----------|-----------------|
| **CUDA** | Implicit, NVIDIA-only | PyTorch, TensorFlow, most ML research |
| **OpenCL** | Portable, explicit-ish | Cross-vendor compute, mobile GPUs |
| **Metal** | Apple-only, encoder model | Apple ML, Core ML, games on macOS/iOS |
| **Vulkan** | Ultra-explicit, portable | AAA games, professional compute |
| **WebGPU** | Safe, browser-first | TensorFlow.js, web ML, browser games |
| **OpenGL** | Legacy state machine | Older software, teaching, compatibility |

## Layer Position

```
Layer 11: Logic Gates (AND, OR, XOR, NAND)
    |
Layer 10: FP Arithmetic (IEEE 754 add/mul/fma)
    |
Layer  9: Accelerator Core (gpu-core) -- one core, one instruction at a time
    |
Layer  8: Parallel Execution Engine -- warps, wavefronts, systolic arrays
    |
Layer  7: Compute Unit -- SM, CU, MXU, XeCore, ANECore
    |
Layer  6: Device Simulator -- complete devices with global memory + work dist.
    |
Layer  5: Compute Runtime -- Vulkan-inspired explicit GPU API
    |
Layer  4: Vendor API Simulators  <-- YOU ARE HERE
    |
    +-->  CUDARuntime       -- "just launch it" (implicit everything)
    +-->  OpenCLRuntime     -- "portable compute" (platform/device/context)
    +-->  MetalRuntime      -- "Apple's way" (command encoders, unified memory)
    +-->  VulkanRuntime     -- "maximum control" (thin wrapper over Layer 5)
    +-->  WebGPURuntime     -- "safe for the web" (single queue, auto sync)
    +-->  OpenGLCompute     -- "the old guard" (global state machine)
    |
Layer  3: Compute Libraries (future -- GEMM, convolution, attention)
    |
Layer  2: Tensor + Autograd (future)
    |
Layer  1: ML Framework (future -- model.fit(), layers, optimizers)
```

**Depends on:**
- `compute-runtime` (Layer 5) -- RuntimeInstance, LogicalDevice, CommandBuffer, etc.
- `device-simulator` (Layer 6) -- AcceleratorDevice, DeviceConfig (transitively)
- `gpu-core` -- Instruction, InstructionSet (transitively)

**Used by:** Compute Libraries (Layer 3, future)

## The Big Picture: Six Ways to Say the Same Thing

All six APIs ultimately do the same four things:

1. **Find a device** and create a usable handle to it
2. **Allocate memory** on the device and transfer data to/from the host
3. **Launch a compute kernel** with some grid of threads
4. **Synchronize** to know when the work is done

The difference is **how much the API hides** from you:

```
    More explicit                                      More implicit
    (you manage everything)                     (runtime manages for you)
    <-------------------------------------------------------------->

    Vulkan      OpenGL      OpenCL      Metal       WebGPU      CUDA
    |           |           |           |           |           |
    Command     Global      Command     Command     Command     Implicit
    buffers     state       queues      encoders    encoders    streams
    |           machine     |           |           |           |
    Explicit    Explicit    Event       Auto        Auto        Auto
    barriers    barriers    chains      barriers    barriers    barriers
    |           |           |           |           |           |
    Memory      Map/Unmap   Buffer      Unified     Auto        Unified
    types       directly    flags       memory      memory      memory
    |           |           |           |           |           |
    Descriptor  SSBO        set_arg()   setBuffer   BindGroup   Direct
    sets        bindings    per-arg     per-index   per-group   args
```

## The Shared Foundation: BaseVendorSimulator

All six simulators share a common internal structure. They all need a Layer 5
runtime instance, a physical device, a logical device, and a memory manager.
Rather than duplicate this setup, we extract it into a base class:

```python
class BaseVendorSimulator:
    """
    ============================================================
    THE COMMON FOUNDATION FOR ALL VENDOR API SIMULATORS
    ============================================================

    Every GPU API, no matter how different its surface looks, needs
    to do the same things underneath:

    1. Find a GPU                  --> RuntimeInstance
    2. Create a usable handle      --> LogicalDevice
    3. Get a queue for submission   --> CommandQueue
    4. Manage memory                --> MemoryManager

    This base class sets all that up. Each simulator subclass then
    adds its vendor-specific vocabulary on top.
    """

    def __init__(self, device_type=None, vendor_hint=None):
        self._instance = RuntimeInstance()
        self._physical_devices = self._instance.enumerate_physical_devices()
        self._physical_device = self._select_device(device_type, vendor_hint)
        self._logical_device = self._instance.create_logical_device(
            self._physical_device
        )
        self._compute_queue = self._logical_device.get_queue("compute", 0)
        self._transfer_queue = self._logical_device.get_queue("transfer", 0)
        self._memory_manager = self._logical_device.memory_manager

    def _select_device(self, device_type, vendor_hint):
        """Pick the best matching device from enumerated physical devices."""
        ...

    def _create_and_submit_cb(self, record_fn):
        """
        Helper: create a command buffer, record commands into it, submit,
        and wait for completion. Used by APIs that hide command buffers
        (CUDA, OpenGL) to provide "immediate" execution semantics.
        """
        cb = self._logical_device.create_command_buffer()
        cb.begin()
        record_fn(cb)
        cb.end()
        fence = self._logical_device.create_fence()
        self._compute_queue.submit([cb], fence=fence)
        fence.wait()
        return cb
```

---

## Simulator 1: CUDA

### Philosophy

CUDA is NVIDIA's proprietary GPU compute API. Its design philosophy is
**"make the common case easy."** Most GPU programs follow the same pattern:
allocate memory, copy data to GPU, launch a kernel, copy results back. CUDA
makes each of those steps a single function call.

The key insight is that CUDA **hides command buffers entirely**. When you write
`kernel<<<grid, block>>>(args)`, CUDA internally creates a command buffer,
records the bind + dispatch, submits it to the default stream, and destroys the
command buffer. You never see any of that. It's like a restaurant where you just
say "steak, medium rare" instead of writing out the full recipe.

### What CUDA Hides vs Layer 5

| Layer 5 (explicit) | CUDA (implicit) |
|---------------------|-----------------|
| Create RuntimeInstance, enumerate devices | Automatic on first CUDA call |
| Choose physical device, create logical device | `cudaSetDevice(0)` |
| Specify memory type flags | Just `cudaMalloc` (always DEVICE_LOCAL) |
| Create CB, begin, record, end, submit, wait | `kernel<<<grid, block>>>()` |
| Create Pipeline, ShaderModule, DescriptorSet | Kernel function + direct args |
| Create Fence, submit with fence, wait | `cudaDeviceSynchronize()` |
| Create CommandQueue with queue family | `cudaStreamCreate()` |
| Explicit pipeline barriers | Automatic within default stream |

### Concept Mapping

```
CUDA                            Layer 5 (Compute Runtime)
----                            -------------------------
cudaSetDevice(0)            --> enumerate_physical_devices()[0]
                                + create_logical_device()
cudaMalloc(&ptr, size)      --> allocate(size, DEVICE_LOCAL)
cudaMallocManaged(&p, size) --> allocate(size, DEVICE_LOCAL|HOST_VISIBLE|HOST_COHERENT)
cudaMemcpy(d, s, n, kind)  --> create CB + cmd_copy_buffer + submit
                                (or map + write + unmap for host<->device)
kernel<<<G, B>>>(args)      --> create Pipeline + DescriptorSet
                                + create CB + bind + dispatch + submit
cudaDeviceSynchronize()     --> logical_device.wait_idle()
cudaStreamCreate()          --> (get additional CommandQueue)
cudaStreamSynchronize(s)    --> fence.wait() on last submission to that stream
cudaEventCreate()           --> create_fence() (used for timing)
cudaFree(ptr)               --> memory_manager.free(buffer)
```

### API Design

```
CUDARuntime(BaseVendorSimulator)
    # Device management
    set_device(device_id: int)
    get_device() -> int
    get_device_properties() -> CUDADeviceProperties
    device_synchronize()
    device_reset()

    # Memory
    malloc(size: int) -> CUDADevicePtr
    malloc_managed(size: int) -> CUDADevicePtr
    free(ptr: CUDADevicePtr)
    memcpy(dst, src, size: int, kind: CUDAMemcpyKind)
    memset(ptr: CUDADevicePtr, value: int, size: int)

    # Kernel launch -- the heart of CUDA
    launch_kernel(kernel: CUDAKernel, grid: dim3, block: dim3,
                  args: list, shared_mem: int = 0,
                  stream: CUDAStream | None = None)

    # Streams (non-default queues)
    create_stream() -> CUDAStream
    destroy_stream(stream: CUDAStream)
    stream_synchronize(stream: CUDAStream)

    # Events (for GPU timing)
    create_event() -> CUDAEvent
    record_event(event: CUDAEvent, stream: CUDAStream | None = None)
    synchronize_event(event: CUDAEvent)
    elapsed_time(start: CUDAEvent, end: CUDAEvent) -> float

CUDAKernel
    code: list[Instruction]        # GPU instructions to execute
    name: str                      # Kernel name for debugging

CUDADevicePtr
    _buffer: Buffer                # Underlying Layer 5 buffer
    device_address: int            # Fake pointer value
    size: int

CUDAStream
    _queue: CommandQueue           # Underlying Layer 5 queue
    _pending_fence: Fence | None   # For synchronization

CUDAEvent
    _fence: Fence                  # Underlying synchronization primitive
    _timestamp: int                # Recorded cycle count

CUDAMemcpyKind: enum
    HostToDevice, DeviceToHost, DeviceToDevice, HostToHost

dim3: namedtuple(x, y, z)         # Grid/block dimensions

CUDADeviceProperties
    name: str
    total_global_mem: int
    shared_mem_per_block: int
    max_threads_per_block: int
    max_grid_size: tuple[int, int, int]
    warp_size: int
    compute_capability: tuple[int, int]
```

### CUDA Example: SAXPY

```python
cuda = CUDARuntime()
print(cuda.get_device_properties().name)  # "NVIDIA H100"

N = 256
alpha = 2.0

# Allocate device memory
d_x = cuda.malloc(N * 4)  # N floats
d_y = cuda.malloc(N * 4)

# Upload data
cuda.memcpy(d_x, host_x_bytes, N * 4, CUDAMemcpyKind.HostToDevice)
cuda.memcpy(d_y, host_y_bytes, N * 4, CUDAMemcpyKind.HostToDevice)

# Launch kernel -- this one call creates CB, pipeline, descriptors,
# records dispatch, submits, all internally
kernel = CUDAKernel(code=saxpy_instructions, name="saxpy")
cuda.launch_kernel(
    kernel,
    grid=dim3(4, 1, 1),       # 4 thread blocks
    block=dim3(64, 1, 1),     # 64 threads per block
    args=[d_x, d_y]
)

# Wait for GPU
cuda.device_synchronize()

# Download results
cuda.memcpy(host_result, d_y, N * 4, CUDAMemcpyKind.DeviceToHost)

# Cleanup
cuda.free(d_x)
cuda.free(d_y)
```

---

## Simulator 2: OpenCL

### Philosophy

OpenCL (Open Computing Language) is Khronos Group's cross-platform compute API.
Its design philosophy is **"write once, run anywhere"** -- the same OpenCL code
runs on NVIDIA GPUs, AMD GPUs, Intel GPUs, and even CPUs. To achieve this
portability, OpenCL uses a **runtime compilation** model where kernel source code
is compiled at runtime for the specific device.

OpenCL has more boilerplate than CUDA but gives you explicit control over
platforms, devices, contexts, and command queues. Every enqueue operation returns
an **event** that can be used for dependency tracking -- you can say "don't run
kernel B until kernel A's event is complete."

### What OpenCL Adds Over Layer 5

| Feature | How OpenCL handles it |
|---------|----------------------|
| Platform abstraction | `CLPlatform` enumerates available implementations |
| Runtime compilation | Source string --> `CLProgram.build()` --> `CLKernel` |
| Event-based deps | Every enqueue returns a `CLEvent`, commands wait on event lists |
| Simpler memory model | Just READ_WRITE / READ_ONLY / WRITE_ONLY flags |
| Automatic work-group sizing | If local_size is None, OpenCL picks optimal |

### Concept Mapping

```
OpenCL                          Layer 5 (Compute Runtime)
------                          -------------------------
clGetPlatformIDs            --> RuntimeInstance (one platform)
clGetDeviceIDs              --> enumerate_physical_devices()
clCreateContext             --> create_logical_device()
clCreateCommandQueue        --> get_queue("compute", 0)
clCreateBuffer              --> allocate(size, memory_type based on flags)
clCreateProgramWithSource   --> store source string
clBuildProgram              --> create ShaderModule
clCreateKernel              --> create Pipeline
clSetKernelArg              --> DescriptorSet.write()
clEnqueueNDRangeKernel      --> create CB + dispatch + submit
clEnqueueReadBuffer         --> cmd_copy_buffer (device->host) or map+read
clEnqueueWriteBuffer        --> cmd_copy_buffer (host->device) or map+write
clWaitForEvents             --> fence.wait() on corresponding fences
clFinish                    --> fence.wait() on last submission
clReleaseMemObject          --> memory_manager.free()
```

### API Design

```
CLPlatform
    get_platforms() -> list[CLPlatform]          # class method
    get_devices(device_type: CLDeviceType) -> list[CLDevice]
    name: str
    vendor: str
    version: str

CLDevice (wraps PhysicalDevice)
    get_info(param: CLDeviceInfo) -> Any
    name: str
    device_type: CLDeviceType
    max_compute_units: int
    max_work_group_size: int
    global_mem_size: int

CLContext
    __init__(devices: list[CLDevice])
    create_buffer(flags: CLMemFlags, size: int, host_ptr: bytes | None) -> CLBuffer
    create_program_with_source(source: str) -> CLProgram
    create_command_queue(device: CLDevice, properties: int = 0) -> CLCommandQueue

CLCommandQueue
    enqueue_nd_range_kernel(kernel: CLKernel, global_size: tuple,
                            local_size: tuple | None = None,
                            wait_list: list[CLEvent] | None = None) -> CLEvent
    enqueue_read_buffer(buffer: CLBuffer, offset: int, size: int,
                        host_ptr: bytearray,
                        wait_list: list[CLEvent] | None = None) -> CLEvent
    enqueue_write_buffer(buffer: CLBuffer, offset: int, size: int,
                         host_ptr: bytes,
                         wait_list: list[CLEvent] | None = None) -> CLEvent
    enqueue_copy_buffer(src: CLBuffer, dst: CLBuffer, size: int,
                        wait_list: list[CLEvent] | None = None) -> CLEvent
    enqueue_fill_buffer(buffer: CLBuffer, pattern: bytes,
                        offset: int, size: int) -> CLEvent
    finish()
    flush()

CLProgram
    build(devices: list[CLDevice] | None = None, options: str = "")
    build_status: CLBuildStatus
    create_kernel(name: str) -> CLKernel

CLKernel
    set_arg(index: int, value: CLBuffer | int | float | bytes)
    name: str

CLBuffer (wraps Buffer)
    size: int
    flags: CLMemFlags

CLEvent (wraps Fence)
    wait()
    status: CLEventStatus

CLMemFlags: enum flags
    READ_WRITE, READ_ONLY, WRITE_ONLY, COPY_HOST_PTR, USE_HOST_PTR, ALLOC_HOST_PTR

CLDeviceType: enum
    GPU, CPU, ACCELERATOR, ALL

CLBuildStatus: enum
    SUCCESS, ERROR, IN_PROGRESS, NONE

CLEventStatus: enum
    QUEUED, SUBMITTED, RUNNING, COMPLETE
```

### OpenCL Example: SAXPY

```python
# Enumerate platforms and devices
platforms = CLPlatform.get_platforms()
devices = platforms[0].get_devices(CLDeviceType.GPU)

# Create context and command queue
ctx = CLContext(devices)
queue = ctx.create_command_queue(devices[0])

# Create buffers
buf_x = ctx.create_buffer(CLMemFlags.READ_ONLY, N * 4)
buf_y = ctx.create_buffer(CLMemFlags.READ_WRITE, N * 4)

# Build program from source, extract kernel
program = ctx.create_program_with_source("saxpy")
program.build()
kernel = program.create_kernel("saxpy")

# Set kernel arguments one by one
kernel.set_arg(0, buf_x)
kernel.set_arg(1, buf_y)

# Upload data, run kernel, download results -- with event dependencies
ev_write_x = queue.enqueue_write_buffer(buf_x, 0, N * 4, host_x_bytes)
ev_write_y = queue.enqueue_write_buffer(buf_y, 0, N * 4, host_y_bytes)
ev_kernel = queue.enqueue_nd_range_kernel(
    kernel, global_size=(N,), local_size=(64,),
    wait_list=[ev_write_x, ev_write_y]
)
ev_read = queue.enqueue_read_buffer(
    buf_y, 0, N * 4, result_bytes,
    wait_list=[ev_kernel]
)
queue.finish()
```

---

## Simulator 3: Metal

### Philosophy

Metal is Apple's GPU API, designed specifically for Apple hardware. Its key
insight is that Apple devices use **unified memory** -- the CPU and GPU share
the same physical RAM. This eliminates the need for explicit host-to-device
copies that CUDA and OpenCL require.

Metal uses a **command encoder** model. Instead of recording commands directly
into a command buffer, you create a specialized encoder (compute, blit, etc.),
record commands into the encoder, then end the encoder. This scoping makes it
clear what type of commands are being recorded.

Metal also eliminates the physical/logical device split. There's just
`MTLDevice` -- you get it, and you use it.

### What Metal Simplifies vs Layer 5

| Layer 5 (explicit) | Metal (simplified) |
|--------------------|--------------------|
| PhysicalDevice + LogicalDevice | Just `MTLDevice` |
| Memory type flags (DEVICE_LOCAL, HOST_VISIBLE, ...) | `storageModeShared` (unified, always) |
| DescriptorSet + layout | `encoder.setBuffer(buf, offset, index)` |
| Explicit map/unmap for host access | `buffer.contents()` always available |
| Create fence for submission sync | `commandBuffer.waitUntilCompleted()` |
| Pipeline barriers with stage flags | Automatic within command buffer |

### Concept Mapping

```
Metal                               Layer 5 (Compute Runtime)
-----                               -------------------------
MTLCreateSystemDefaultDevice()  --> enumerate_physical_devices() + create_logical_device()
device.makeCommandQueue()       --> get_queue("compute", 0)
queue.makeCommandBuffer()       --> create_command_buffer()
cb.makeComputeCommandEncoder()  --> cb.begin() (scoped to compute)
encoder.setComputePipelineState --> cb.cmd_bind_pipeline()
encoder.setBuffer(b, off, idx)  --> descriptor_set.write() + cb.cmd_bind_descriptor_set()
encoder.dispatchThreadgroups    --> cb.cmd_dispatch()
encoder.endEncoding()           --> (part of recording scope)
cb.commit()                     --> queue.submit([cb])
cb.waitUntilCompleted()         --> fence.wait()
device.makeBuffer(length:)      --> allocate(size, DEVICE_LOCAL|HOST_VISIBLE|HOST_COHERENT)
buffer.contents()               --> map() (always available on unified memory)
device.makeLibrary(source:)     --> create ShaderModule
library.makeFunction(name:)     --> (extract entry point)
device.makeComputePipelineState --> create Pipeline
```

### API Design

```
MTLDevice (wraps BaseVendorSimulator with unified memory)
    name: str
    make_command_queue() -> MTLCommandQueue
    make_buffer(length: int, options: MTLResourceOptions = .storageModeShared) -> MTLBuffer
    make_library(source: str) -> MTLLibrary
    make_compute_pipeline_state(function: MTLFunction) -> MTLComputePipelineState

MTLCommandQueue
    make_command_buffer() -> MTLCommandBuffer

MTLCommandBuffer
    make_compute_command_encoder() -> MTLComputeCommandEncoder
    make_blit_command_encoder() -> MTLBlitCommandEncoder
    commit()                    # submits to queue
    wait_until_completed()      # blocks until done
    status: MTLCommandBufferStatus
    add_completed_handler(handler: Callable)

MTLComputeCommandEncoder
    set_compute_pipeline_state(pso: MTLComputePipelineState)
    set_buffer(buffer: MTLBuffer, offset: int, index: int)
    set_bytes(data: bytes, index: int)    # push constants
    dispatch_threadgroups(groups: MTLSize, threads_per_group: MTLSize)
    dispatch_threads(threads: MTLSize, threads_per_group: MTLSize)
    end_encoding()

MTLBlitCommandEncoder
    copy_from_buffer(src: MTLBuffer, src_offset: int,
                     to_buffer: MTLBuffer, dst_offset: int, size: int)
    fill_buffer(buffer: MTLBuffer, range: range, value: int)
    end_encoding()

MTLBuffer
    contents() -> bytearray    # always available (unified memory)
    length: int
    write_bytes(data: bytes, offset: int = 0)   # convenience

MTLLibrary
    make_function(name: str) -> MTLFunction

MTLFunction (wraps ShaderModule)
    name: str

MTLComputePipelineState (wraps Pipeline)
    max_total_threads_per_threadgroup: int

MTLSize: namedtuple(width, height, depth)

MTLResourceOptions: enum
    storageModeShared      # CPU + GPU access (default on Apple)
    storageModePrivate     # GPU only (for pure GPU buffers)
    storageModeManaged     # CPU + GPU with explicit sync (macOS only)

MTLCommandBufferStatus: enum
    notEnqueued, enqueued, committed, scheduled, completed, error
```

### Metal Example: SAXPY

```python
device = MTLDevice()
queue = device.make_command_queue()

# Allocate -- unified memory, no host/device distinction
buf_x = device.make_buffer(N * 4)
buf_y = device.make_buffer(N * 4)

# Write directly -- no memcpy needed!
buf_x.write_bytes(host_x_bytes)
buf_y.write_bytes(host_y_bytes)

# Create pipeline
library = device.make_library(source="saxpy")
function = library.make_function("saxpy")
pso = device.make_compute_pipeline_state(function)

# Encode and submit
cb = queue.make_command_buffer()
encoder = cb.make_compute_command_encoder()
encoder.set_compute_pipeline_state(pso)
encoder.set_buffer(buf_x, offset=0, index=0)
encoder.set_buffer(buf_y, offset=0, index=1)
encoder.dispatch_threadgroups(MTLSize(4, 1, 1), threads_per_group=MTLSize(64, 1, 1))
encoder.end_encoding()
cb.commit()
cb.wait_until_completed()

# Read directly -- no memcpy needed!
result = bytes(buf_y.contents())
```

---

## Simulator 4: Vulkan

### Philosophy

Vulkan is the most explicit GPU API. You manage everything: memory types,
command buffer recording, queue submission, synchronization barriers, descriptor
set layouts. The reward for this verbosity is **maximum control and performance**.

Since our Layer 5 is already Vulkan-inspired, this simulator is the **thinnest
wrapper**. It adds Vulkan naming conventions (the `vk_` prefix), Vulkan-specific
structures (VkSubmitInfo, VkBufferCreateInfo), and VkResult return codes. The
actual functionality is delegated directly to Layer 5.

### What This Wrapper Adds

| Feature | Implementation |
|---------|---------------|
| Vulkan naming conventions | `vk_create_buffer()` instead of `allocate()` |
| VkResult return codes | Return VkResult instead of raising exceptions |
| Create-info structures | VkBufferCreateInfo, VkMemoryAllocateInfo, etc. |
| Command pools | Groups command buffers (thin wrapper) |
| Pipeline bind points | VkPipelineBindPoint.COMPUTE |

### API Design

```
VkInstance (wraps RuntimeInstance)
    vk_enumerate_physical_devices() -> list[VkPhysicalDevice]

VkPhysicalDevice (wraps PhysicalDevice)
    vk_get_physical_device_properties() -> VkPhysicalDeviceProperties
    vk_get_physical_device_memory_properties() -> VkPhysicalDeviceMemoryProperties
    vk_get_physical_device_queue_family_properties() -> list[VkQueueFamilyProperties]

VkDevice (wraps LogicalDevice)
    vk_get_device_queue(family_index: int, queue_index: int) -> VkQueue
    vk_create_command_pool(create_info: VkCommandPoolCreateInfo) -> VkCommandPool
    vk_allocate_memory(alloc_info: VkMemoryAllocateInfo) -> VkDeviceMemory
    vk_create_buffer(create_info: VkBufferCreateInfo) -> VkBuffer
    vk_bind_buffer_memory(buffer: VkBuffer, memory: VkDeviceMemory, offset: int)
    vk_map_memory(memory: VkDeviceMemory, offset: int, size: int) -> memoryview
    vk_unmap_memory(memory: VkDeviceMemory)
    vk_create_shader_module(create_info: VkShaderModuleCreateInfo) -> VkShaderModule
    vk_create_compute_pipelines(create_infos: list) -> list[VkPipeline]
    vk_create_descriptor_set_layout(create_info) -> VkDescriptorSetLayout
    vk_create_pipeline_layout(create_info) -> VkPipelineLayout
    vk_allocate_descriptor_sets(alloc_info) -> list[VkDescriptorSet]
    vk_update_descriptor_sets(writes: list[VkWriteDescriptorSet])
    vk_create_fence(flags: int = 0) -> VkFence
    vk_create_semaphore() -> VkSemaphore
    vk_wait_for_fences(fences: list[VkFence], wait_all: bool, timeout: int) -> VkResult
    vk_reset_fences(fences: list[VkFence])
    vk_device_wait_idle()

VkCommandPool (groups command buffers)
    vk_allocate_command_buffers(count: int) -> list[VkCommandBuffer]
    vk_reset_command_pool()
    vk_free_command_buffers(buffers: list[VkCommandBuffer])

VkCommandBuffer (wraps CommandBuffer)
    vk_begin_command_buffer(flags: int = 0)
    vk_end_command_buffer()
    vk_cmd_bind_pipeline(bind_point: VkPipelineBindPoint, pipeline: VkPipeline)
    vk_cmd_bind_descriptor_sets(bind_point, layout, sets: list[VkDescriptorSet])
    vk_cmd_push_constants(layout, offset: int, data: bytes)
    vk_cmd_dispatch(x: int, y: int, z: int)
    vk_cmd_copy_buffer(src: VkBuffer, dst: VkBuffer, regions: list[VkBufferCopy])
    vk_cmd_fill_buffer(buffer: VkBuffer, offset: int, size: int, data: int)
    vk_cmd_pipeline_barrier(src_stage, dst_stage, buffer_barriers: list)

VkQueue (wraps CommandQueue)
    vk_queue_submit(submits: list[VkSubmitInfo], fence: VkFence | None) -> VkResult
    vk_queue_wait_idle()

# Structures
VkBufferCreateInfo(size, usage, sharing_mode)
VkMemoryAllocateInfo(size, memory_type_index)
VkShaderModuleCreateInfo(code)
VkComputePipelineCreateInfo(shader_stage, layout)
VkSubmitInfo(command_buffers, wait_semaphores, signal_semaphores)
VkBufferCopy(src_offset, dst_offset, size)
VkWriteDescriptorSet(dst_set, dst_binding, descriptor_type, buffer_info)
VkDescriptorBufferInfo(buffer, offset, range)
VkPipelineShaderStageCreateInfo(stage, module, entry_point)

VkResult: enum
    SUCCESS, NOT_READY, TIMEOUT, ERROR_OUT_OF_DEVICE_MEMORY,
    ERROR_DEVICE_LOST, ERROR_INITIALIZATION_FAILED

VkPipelineBindPoint: enum
    COMPUTE

VkBufferUsageFlagBits: flags
    STORAGE_BUFFER, UNIFORM_BUFFER, TRANSFER_SRC, TRANSFER_DST

VkMemoryPropertyFlagBits: flags
    DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT, HOST_CACHED
```

### Vulkan Example: SAXPY

```python
# Create instance and device (verbose but explicit)
instance = VkInstance()
physical_devices = instance.vk_enumerate_physical_devices()
device = instance.vk_create_device(physical_devices[0])
queue = device.vk_get_device_queue(family_index=0, queue_index=0)

# Allocate memory and create buffers
mem_x = device.vk_allocate_memory(VkMemoryAllocateInfo(
    size=N * 4, memory_type_index=0))  # DEVICE_LOCAL
buf_x = device.vk_create_buffer(VkBufferCreateInfo(
    size=N * 4, usage=VkBufferUsageFlagBits.STORAGE_BUFFER))
device.vk_bind_buffer_memory(buf_x, mem_x, offset=0)

# Upload via staging buffer (discrete GPU path)
staging = device.vk_allocate_memory(VkMemoryAllocateInfo(
    size=N * 4, memory_type_index=1))  # HOST_VISIBLE
staging_buf = device.vk_create_buffer(VkBufferCreateInfo(
    size=N * 4, usage=VkBufferUsageFlagBits.TRANSFER_SRC))
device.vk_bind_buffer_memory(staging_buf, staging, offset=0)
mapped = device.vk_map_memory(staging, 0, N * 4)
mapped[:] = host_x_bytes
device.vk_unmap_memory(staging)

# Create shader, pipeline, descriptor set, record, submit...
# (most verbose of all 6 APIs)
```

---

## Simulator 5: WebGPU

### Philosophy

WebGPU is the modern web GPU API, designed to be **safe, portable, and simple**.
It runs in browsers on top of Vulkan (Linux/Windows/Android), Metal (macOS/iOS),
or D3D12 (Windows). The API is designed so that **you cannot cause undefined
behavior** -- every operation is validated, and the runtime manages
synchronization automatically.

Key simplifications over Vulkan:
- **Single queue** -- `device.queue` is all you get
- **Automatic barriers** -- no manual synchronization
- **No memory types** -- just usage flags, the runtime picks memory
- **Always validated** -- no optional validation layers
- **WGSL shaders** -- WebGPU's own shading language

### Concept Mapping

```
WebGPU                              Layer 5 (Compute Runtime)
------                              -------------------------
navigator.gpu.requestAdapter()  --> RuntimeInstance.enumerate_physical_devices()
adapter.requestDevice()         --> create_logical_device()
device.queue                    --> get_queue("compute", 0) (single queue)
device.createBuffer(desc)       --> allocate(size, auto-selected memory type)
device.createShaderModule(code) --> create ShaderModule
device.createComputePipeline()  --> create Pipeline
device.createBindGroup()        --> create DescriptorSet
device.createCommandEncoder()   --> create CommandBuffer + begin()
encoder.beginComputePass()      --> (scoping, no Layer 5 equivalent)
pass.setPipeline()              --> cmd_bind_pipeline()
pass.setBindGroup()             --> cmd_bind_descriptor_set()
pass.dispatchWorkgroups()       --> cmd_dispatch()
pass.end()                      --> (end scope)
encoder.finish()                --> cb.end() -> returns frozen GPUCommandBuffer
device.queue.submit([cb])       --> queue.submit([cb])
device.queue.writeBuffer()      --> map + write + unmap (convenience)
buffer.mapAsync() + getMapped() --> map() (simulated as sync for simplicity)
```

### API Design

```
GPU
    request_adapter(options: GPURequestAdapterOptions | None = None) -> GPUAdapter

GPUAdapter
    name: str
    features: set[str]
    limits: GPUAdapterLimits
    request_device(descriptor: GPUDeviceDescriptor | None = None) -> GPUDevice

GPUDevice
    queue: GPUQueue                     # single queue, always available
    features: set[str]
    limits: GPUDeviceLimits

    create_buffer(descriptor: GPUBufferDescriptor) -> GPUBuffer
    create_shader_module(descriptor: GPUShaderModuleDescriptor) -> GPUShaderModule
    create_compute_pipeline(descriptor: GPUComputePipelineDescriptor) -> GPUComputePipeline
    create_bind_group(descriptor: GPUBindGroupDescriptor) -> GPUBindGroup
    create_bind_group_layout(descriptor: GPUBindGroupLayoutDescriptor) -> GPUBindGroupLayout
    create_pipeline_layout(descriptor: GPUPipelineLayoutDescriptor) -> GPUPipelineLayout
    create_command_encoder(descriptor: GPUCommandEncoderDescriptor | None = None) -> GPUCommandEncoder
    destroy()

GPUQueue
    submit(command_buffers: list[GPUCommandBuffer])
    write_buffer(buffer: GPUBuffer, buffer_offset: int, data: bytes)

GPUCommandEncoder
    begin_compute_pass(descriptor: GPUComputePassDescriptor | None = None) -> GPUComputePassEncoder
    copy_buffer_to_buffer(src: GPUBuffer, src_offset: int,
                          dst: GPUBuffer, dst_offset: int, size: int)
    finish() -> GPUCommandBuffer

GPUComputePassEncoder
    set_pipeline(pipeline: GPUComputePipeline)
    set_bind_group(index: int, bind_group: GPUBindGroup)
    dispatch_workgroups(x: int, y: int = 1, z: int = 1)
    end()

GPUBuffer
    size: int
    usage: int  # GPUBufferUsage flags
    map_async(mode: GPUMapMode, offset: int = 0, size: int | None = None)
    get_mapped_range(offset: int = 0, size: int | None = None) -> bytearray
    unmap()
    destroy()

GPUShaderModule (wraps ShaderModule)
GPUComputePipeline (wraps Pipeline)
    get_bind_group_layout(index: int) -> GPUBindGroupLayout
GPUBindGroup (wraps DescriptorSet)
GPUBindGroupLayout (wraps DescriptorSetLayout)
GPUCommandBuffer (frozen, immutable after finish())

GPUBufferUsage: flags
    MAP_READ, MAP_WRITE, COPY_SRC, COPY_DST, STORAGE, UNIFORM

GPUMapMode: flags
    READ, WRITE

GPUBindGroupEntry(binding: int, resource: GPUBuffer)
GPUBindGroupLayoutEntry(binding: int, visibility: int, buffer: GPUBufferBindingLayout)
GPUBufferBindingLayout(type: str = "storage")
GPUProgrammableStage(module: GPUShaderModule, entry_point: str)
GPUComputePipelineDescriptor(layout, compute: GPUProgrammableStage)
GPUBufferDescriptor(size: int, usage: int, mapped_at_creation: bool = False)
```

### WebGPU Example: SAXPY

```python
gpu = GPU()
adapter = gpu.request_adapter()
device = adapter.request_device()

# Create buffers -- no memory type selection needed
buf_x = device.create_buffer(GPUBufferDescriptor(
    size=N * 4,
    usage=GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST
))
buf_y = device.create_buffer(GPUBufferDescriptor(
    size=N * 4,
    usage=GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC | GPUBufferUsage.COPY_DST
))

# Upload -- convenience method, no staging buffers
device.queue.write_buffer(buf_x, 0, host_x_bytes)
device.queue.write_buffer(buf_y, 0, host_y_bytes)

# Create pipeline
shader = device.create_shader_module(GPUShaderModuleDescriptor(code="saxpy"))
pipeline = device.create_compute_pipeline(GPUComputePipelineDescriptor(
    layout="auto",
    compute=GPUProgrammableStage(module=shader, entry_point="main")
))

# Create bind group
bind_group = device.create_bind_group(GPUBindGroupDescriptor(
    layout=pipeline.get_bind_group_layout(0),
    entries=[
        GPUBindGroupEntry(binding=0, resource=buf_x),
        GPUBindGroupEntry(binding=1, resource=buf_y),
    ]
))

# Encode commands -- note the compute pass scoping
encoder = device.create_command_encoder()
compute_pass = encoder.begin_compute_pass()
compute_pass.set_pipeline(pipeline)
compute_pass.set_bind_group(0, bind_group)
compute_pass.dispatch_workgroups(4)
compute_pass.end()
cb = encoder.finish()

# Submit -- automatic synchronization, no fences needed
device.queue.submit([cb])
```

---

## Simulator 6: OpenGL Compute

### Philosophy

OpenGL is the oldest surviving GPU API (1992). Compute shaders were bolted on
in OpenGL 4.3 (2012), long after the core API was designed around graphics
rendering. This heritage shows: OpenGL uses a **global state machine** model
where you bind things to the "current" state and then issue commands that
operate on whatever is currently bound.

There are no explicit command buffers in OpenGL. Every call is **immediate** --
when you call `glDispatchCompute()`, it executes right away (well, it gets
queued internally, but from the API perspective it's immediate). This makes
OpenGL the simplest API to use for small programs but the hardest to optimize
for complex workloads.

### What OpenGL's State Machine Means

```
The OpenGL Way (global state):        The Vulkan Way (explicit objects):

  glUseProgram(prog)     # global      cb.cmd_bind_pipeline(pipeline)
  glBindBufferBase(0, a) # global      ds.write(binding=0, buffer=a)
  glBindBufferBase(1, b) # global      ds.write(binding=1, buffer=b)
  glDispatchCompute(4,1,1) # uses      cb.cmd_bind_descriptor_set(ds)
                           # whatever  cb.cmd_dispatch(4, 1, 1)
                           # is bound  queue.submit([cb])

State is IMPLICIT.                     State is EXPLICIT.
You hope you bound the right things.   You KNOW what's bound because you said so.
```

### Concept Mapping

```
OpenGL                              Layer 5 (Compute Runtime)
------                              -------------------------
(implicit context)              --> RuntimeInstance + create_logical_device()
glCreateProgram/glLinkProgram   --> create Pipeline
glCreateShader(GL_COMPUTE)      --> create ShaderModule
glGenBuffers/glBufferData       --> allocate()
glBindBufferBase(SSBO, idx, b)  --> DescriptorSet.write(binding=idx, buffer=b)
glUseProgram(prog)              --> (store as current program in state)
glDispatchCompute(x, y, z)      --> create CB + bind pipeline + bind desc + dispatch + submit
glMemoryBarrier(bits)           --> cmd_pipeline_barrier()
glMapBufferRange                --> map()
glUnmapBuffer                   --> unmap()
glFinish()                      --> logical_device.wait_idle()
glFenceSync/glClientWaitSync    --> Fence create + wait
glDeleteBuffers                 --> free()
```

### API Design

```
GLContext (wraps BaseVendorSimulator + global state)
    # Internal state
    _current_program: GLuint | None
    _bound_buffers: dict[tuple[GLenum, int], GLuint]  # (target, index) -> buffer
    _programs: dict[GLuint, Pipeline]
    _shaders: dict[GLuint, ShaderModule]
    _buffers: dict[GLuint, Buffer]
    _next_id: int  # GL uses integer handles for everything

    # Shader/Program management
    create_shader(shader_type: int) -> int
    shader_source(shader: int, source: str)
    compile_shader(shader: int)
    create_program() -> int
    attach_shader(program: int, shader: int)
    link_program(program: int)
    use_program(program: int)
    delete_program(program: int)
    delete_shader(shader: int)

    # Buffer management
    gen_buffers(count: int) -> list[int]
    delete_buffers(buffers: list[int])
    bind_buffer(target: int, buffer: int)
    buffer_data(target: int, size: int, data: bytes | None, usage: int)
    buffer_sub_data(target: int, offset: int, data: bytes)
    bind_buffer_base(target: int, index: int, buffer: int)
    map_buffer_range(target: int, offset: int, length: int, access: int) -> bytearray
    unmap_buffer(target: int) -> bool

    # Compute dispatch
    dispatch_compute(num_groups_x: int, num_groups_y: int, num_groups_z: int)

    # Synchronization
    memory_barrier(barriers: int)
    fence_sync() -> int                # returns sync object handle
    client_wait_sync(sync: int, flags: int, timeout: int) -> int
    delete_sync(sync: int)
    finish()

    # Uniforms (push constants)
    get_uniform_location(program: int, name: str) -> int
    uniform_1f(location: int, value: float)
    uniform_1i(location: int, value: int)

# Constants (module-level, like real OpenGL)
GL_COMPUTE_SHADER = 0x91B9
GL_SHADER_STORAGE_BUFFER = 0x90D2
GL_DYNAMIC_DRAW = 0x88E8
GL_STATIC_DRAW = 0x88E4
GL_MAP_READ_BIT = 0x0001
GL_MAP_WRITE_BIT = 0x0002
GL_SHADER_STORAGE_BARRIER_BIT = 0x00002000
GL_BUFFER_UPDATE_BARRIER_BIT = 0x00000200
GL_ALL_BARRIER_BITS = 0xFFFFFFFF
GL_ALREADY_SIGNALED = 0x911A
GL_CONDITION_SATISFIED = 0x911C
GL_TIMEOUT_EXPIRED = 0x911B
GL_WAIT_FAILED = 0x911D
```

### OpenGL Example: SAXPY

```python
gl = GLContext()

# Create compute shader and program
shader = gl.create_shader(GL_COMPUTE_SHADER)
gl.shader_source(shader, "saxpy")
gl.compile_shader(shader)
program = gl.create_program()
gl.attach_shader(program, shader)
gl.link_program(program)

# Create and fill buffers
bufs = gl.gen_buffers(2)
gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
gl.buffer_data(GL_SHADER_STORAGE_BUFFER, N * 4, host_x_bytes, GL_DYNAMIC_DRAW)
gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[1])
gl.buffer_data(GL_SHADER_STORAGE_BUFFER, N * 4, host_y_bytes, GL_DYNAMIC_DRAW)

# Bind to indexed SSBO binding points
gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])
gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 1, bufs[1])

# Dispatch (uses whatever program and buffers are currently bound)
gl.use_program(program)
gl.dispatch_compute(4, 1, 1)
gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT)

# Read back
gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[1])
result = gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, N * 4, GL_MAP_READ_BIT)
gl.unmap_buffer(GL_SHADER_STORAGE_BUFFER)

gl.finish()
```

---

## Cross-API Equivalence Test

The capstone test for this package is running the **same computation through all
six APIs** and verifying that all six produce the same result. This proves that
our simulators are functionally equivalent wrappers over the same Layer 5 engine:

```python
def test_saxpy_all_apis():
    """Run SAXPY through CUDA, OpenCL, Metal, Vulkan, WebGPU, and OpenGL.
    All six must produce identical results."""

    N = 64
    x_data = [float(i) for i in range(N)]
    y_data = [float(i * 2) for i in range(N)]
    alpha = 2.0
    expected = [alpha * x + y for x, y in zip(x_data, y_data)]

    results = {}
    results["cuda"] = run_saxpy_cuda(x_data, y_data, alpha)
    results["opencl"] = run_saxpy_opencl(x_data, y_data, alpha)
    results["metal"] = run_saxpy_metal(x_data, y_data, alpha)
    results["vulkan"] = run_saxpy_vulkan(x_data, y_data, alpha)
    results["webgpu"] = run_saxpy_webgpu(x_data, y_data, alpha)
    results["opengl"] = run_saxpy_opengl(x_data, y_data, alpha)

    for api_name, result in results.items():
        assert result == expected, f"{api_name} produced wrong results"
```

## Testing Strategy

Each simulator needs:

1. **Device discovery** -- create runtime, find device, query properties
2. **Memory management** -- allocate, free, write, read, copy
3. **Kernel dispatch** -- bind kernel, dispatch, verify execution
4. **Multiple dispatches** -- sequential kernels with data dependencies
5. **Error handling** -- invalid operations, freed resources, state violations
6. **API-specific features**:
   - CUDA: streams, events, unified memory, memset
   - OpenCL: program build, event chains, flush vs finish
   - Metal: command encoders, blit encoder, unified memory access
   - Vulkan: all create-info structures, command pools, VkResult codes
   - WebGPU: buffer mapping, auto bind group layout, single queue
   - OpenGL: state tracking, SSBO binding, memory barriers, sync objects

Target: 30+ tests per simulator, 95%+ coverage.

## Package Structure

```
vendor-api-simulators/
    src/vendor_api_simulators/
        __init__.py
        _base.py              # BaseVendorSimulator
        cuda.py               # CUDARuntime + supporting types
        opencl.py             # CLPlatform, CLContext, etc.
        metal.py              # MTLDevice, MTLCommandBuffer, etc.
        vulkan.py             # VkInstance, VkDevice, etc.
        webgpu.py             # GPU, GPUAdapter, GPUDevice, etc.
        opengl.py             # GLContext + GL constants
    tests/
        test_cuda.py
        test_opencl.py
        test_metal.py
        test_vulkan.py
        test_webgpu.py
        test_opengl.py
        test_cross_api.py     # same computation through all 6
```

## Implementation Order

1. `_base.py` -- shared foundation
2. `vulkan.py` -- thinnest wrapper, validates base works
3. `cuda.py` -- most common API, validates "hide everything" pattern
4. `metal.py` -- validates unified memory path
5. `opencl.py` -- validates runtime compilation model
6. `webgpu.py` -- validates single-queue + auto-sync model
7. `opengl.py` -- validates global state machine pattern
8. `test_cross_api.py` -- the capstone integration test
