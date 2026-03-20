# Vendor API Simulators

Six vendor GPU API simulators (CUDA, OpenCL, Metal, Vulkan, WebGPU, OpenGL) implemented as thin wrappers over the Vulkan-inspired compute runtime (Layer 5). Each simulator translates its vendor-specific API calls into the common low-level operations underneath.

## Layer Position

```
Layer 5: Compute Runtime (Vulkan-inspired)
    |
Layer 4: Vendor API Simulators  <-- THIS PACKAGE
    |
    +--> CUDARuntime       -- "just launch it" (implicit everything)
    +--> OpenCLRuntime     -- "portable compute" (platform/device/context)
    +--> MetalRuntime      -- "Apple's way" (command encoders, unified memory)
    +--> VulkanRuntime     -- "maximum control" (thin wrapper over Layer 5)
    +--> WebGPURuntime     -- "safe for the web" (single queue, auto sync)
    +--> OpenGLCompute     -- "the old guard" (global state machine)
```

## Installation

```bash
pip install coding-adventures-vendor-api-simulators
```

## Quick Examples

### CUDA (simplest)

```python
from vendor_api_simulators.cuda import CUDARuntime, CUDAKernel, dim3, CUDAMemcpyKind
from gpu_core import limm, halt

cuda = CUDARuntime()
d_x = cuda.malloc(256)
kernel = CUDAKernel(code=[limm(0, 42.0), halt()], name="test")
cuda.launch_kernel(kernel, grid=dim3(1, 1, 1), block=dim3(32, 1, 1), args=[d_x])
cuda.device_synchronize()
cuda.free(d_x)
```

### Metal (unified memory)

```python
from vendor_api_simulators.metal import MTLDevice, MTLSize

device = MTLDevice()
queue = device.make_command_queue()
buf = device.make_buffer(256)
buf.write_bytes(b'\x00' * 256)
result = bytes(buf.contents())
```

### OpenGL (state machine)

```python
from vendor_api_simulators.opengl import GLContext, GL_COMPUTE_SHADER, GL_SHADER_STORAGE_BUFFER

gl = GLContext()
shader = gl.create_shader(GL_COMPUTE_SHADER)
gl.shader_source(shader, "test")
gl.compile_shader(shader)
```

## Dependencies

- `coding-adventures-compute-runtime` (Layer 5)
