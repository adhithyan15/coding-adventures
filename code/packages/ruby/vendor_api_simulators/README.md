# Vendor API Simulators (Ruby)

**Layer 3** of the accelerator computing stack — six GPU vendor API simulators, all built on top of the Layer 5 compute runtime.

## What is this?

This package provides Ruby implementations of six real-world GPU programming APIs:

| API     | Style                  | Vendor    | Key Concept                        |
|---------|------------------------|-----------|------------------------------------|
| CUDA    | Implicit/stream-based  | NVIDIA    | `malloc` → `memcpy` → `launch`    |
| OpenCL  | Portable/event-based   | Any       | Programs, kernels, event deps      |
| Metal   | Apple/encoder-based    | Apple     | Unified memory, command encoders   |
| Vulkan  | Ultra-explicit         | Any       | Everything is a create-info struct |
| WebGPU  | Browser-safe           | Any       | Single queue, descriptor model     |
| OpenGL  | Legacy state machine   | Any       | Global state, integer handles      |

Each simulator is a thin wrapper over the **compute runtime** (Layer 5). They all share the same underlying engine — the same "kitchen" behind six different "restaurant fronts."

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Layer 3: Vendor API Simulators (this package)  │
│  CUDA · OpenCL · Metal · Vulkan · WebGPU · GL   │
├─────────────────────────────────────────────────┤
│  Layer 5: Compute Runtime                        │
│  RuntimeInstance · LogicalDevice · CommandBuffer  │
│  MemoryManager · Pipeline · Fence · Queue        │
└─────────────────────────────────────────────────┘
```

All six simulators extend `BaseVendorSimulator`, which handles:
- Device discovery and selection (4-pass vendor/type matching)
- Logical device creation
- Queue and memory manager setup
- The `_create_and_submit_cb` helper for implicit-execution APIs

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_vendor_api_simulators", path: "code/packages/ruby/vendor_api_simulators"
```

Then `bundle install`.

## Usage

### CUDA

```ruby
include CodingAdventures::VendorApiSimulators

cuda = CUDARuntime.new
ptr = cuda.malloc(1024)
cuda.memcpy(ptr, data, 1024, :host_to_device)
kernel = CUDAKernel.new(code: nil, name: "my_kernel")
cuda.launch_kernel(kernel, grid: Dim3.new(x: 4), block: Dim3.new(x: 64), args: [ptr])
cuda.device_synchronize
cuda.free(ptr)
```

### OpenCL

```ruby
ctx = CLContext.new
queue = ctx.create_command_queue
buf = ctx.create_buffer(CLMemFlags::READ_WRITE, 256)
prog = ctx.create_program_with_source("my_source")
prog.build
kernel = prog.create_kernel("my_kernel")
kernel.set_arg(0, buf)
queue.enqueue_nd_range_kernel(kernel, [128], local_size: [64])
queue.finish
```

### Metal

```ruby
device = MTLDevice.new
queue = device.make_command_queue
buf = device.make_buffer(256)
func = MTLFunction.new("my_func")
pso = device.make_compute_pipeline_state(func)

cb = queue.make_command_buffer
encoder = cb.make_compute_command_encoder
encoder.set_compute_pipeline_state(pso)
encoder.set_buffer(buf, offset: 0, index: 0)
encoder.dispatch_threadgroups(MTLSize.new(width: 4), MTLSize.new(width: 64))
encoder.end_encoding
cb.commit
cb.wait_until_completed
```

### Vulkan

```ruby
instance = VkInstance.new
pdevs = instance.vk_enumerate_physical_devices
device = instance.vk_create_device(pdevs[0])
queue = device.vk_get_device_queue(0, 0)
# ... (Vulkan is verbose by design — see tests for full examples)
```

### WebGPU

```ruby
gpu = GPU.new
adapter = gpu.request_adapter
device = adapter.request_device
buf = device.create_buffer(GPUBufferDescriptor.new(size: 256, usage: GPUBufferUsage::STORAGE))
# ... (see tests for full pipeline setup)
```

### OpenGL

```ruby
gl = GLContext.new
shader = gl.create_shader(GL_COMPUTE_SHADER)
gl.shader_source(shader, "my_shader")
gl.compile_shader(shader)
prog = gl.create_program
gl.attach_shader(prog, shader)
gl.link_program(prog)
gl.use_program(prog)
gl.dispatch_compute(4, 1, 1)
gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT)
```

## Testing

```bash
cd code/packages/ruby/vendor_api_simulators
bundle install --quiet
bundle exec rake test
```

## Dependencies

- `coding_adventures_compute_runtime` (Layer 5) — the underlying compute engine

## License

MIT
