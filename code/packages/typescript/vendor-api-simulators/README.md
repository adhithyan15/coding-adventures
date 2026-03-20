# @coding-adventures/vendor-api-simulators

Six GPU vendor API simulators, each wrapping the same Vulkan-inspired compute runtime (Layer 5) with different programming models.

## Simulators

| API | Model | Key Concept |
|-----|-------|-------------|
| **CUDA** | Implicit, "just launch it" | Streams, events, dim3 grid/block |
| **OpenCL** | Cross-platform, event-based | Platform/device/context hierarchy |
| **Metal** | Unified memory, command encoders | MTLBuffer.contents() gives CPU access |
| **Vulkan** | Ultra-explicit, maximum control | Create-info structs, VkResult codes |
| **WebGPU** | Safe, browser-first, single queue | Bind groups, command encoder -> frozen buffer |
| **OpenGL** | Legacy global state machine | Integer handles, glBind* + glDispatch* |

## Installation

```bash
npm install @coding-adventures/vendor-api-simulators
```

## Quick Start

### CUDA (simplest)

```typescript
import { CUDARuntime, makeCUDAKernel, makeDim3, CUDAMemcpyKind } from "@coding-adventures/vendor-api-simulators";

const cuda = new CUDARuntime();
const d_x = cuda.malloc(256);
const kernel = makeCUDAKernel(instructions, "saxpy");
cuda.launchKernel(kernel, makeDim3(1, 1, 1), makeDim3(32, 1, 1), [d_x]);
cuda.deviceSynchronize();
cuda.free(d_x);
```

### Metal (unified memory)

```typescript
import { MTLDevice, makeMTLSize } from "@coding-adventures/vendor-api-simulators";

const device = new MTLDevice();
const buf = device.makeBuffer(256);
buf.writeBytes(data);
// GPU computes on buf...
const result = buf.contents(); // CPU reads directly, no copy needed
```

### WebGPU (browser-style)

```typescript
import { GPU, GPUBufferUsage, GPUMapMode } from "@coding-adventures/vendor-api-simulators";

const gpu = new GPU();
const adapter = gpu.requestAdapter();
const device = adapter.requestDevice();
const buffer = device.createBuffer({ size: 256, usage: GPUBufferUsage.STORAGE });
device.queue.writeBuffer(buffer, 0, data);
```

### Vulkan (explicit)

```typescript
import { VkInstance, VkBufferUsageFlagBits, VkSharingMode } from "@coding-adventures/vendor-api-simulators";

const instance = new VkInstance();
const physicals = instance.vkEnumeratePhysicalDevices();
const device = instance.vkCreateDevice(physicals[0]);
const buffer = device.vkCreateBuffer({
  size: 256,
  usage: VkBufferUsageFlagBits.STORAGE_BUFFER,
  sharingMode: VkSharingMode.EXCLUSIVE,
});
```

### OpenGL (state machine)

```typescript
import { GLContext, GL_COMPUTE_SHADER, GL_SHADER_STORAGE_BUFFER, GL_STATIC_DRAW } from "@coding-adventures/vendor-api-simulators";

const gl = new GLContext();
const shader = gl.createShader(GL_COMPUTE_SHADER);
gl.shaderSource(shader, "compute_src");
gl.compileShader(shader);
const prog = gl.createProgram();
gl.attachShader(prog, shader);
gl.linkProgram(prog);
gl.useProgram(prog);
gl.dispatchCompute(4, 1, 1);
```

## Architecture

All six simulators extend `BaseVendorSimulator`, which handles:
- Device discovery via `RuntimeInstance.enumeratePhysicalDevices()`
- Logical device creation with compute queue
- Memory manager access
- A `_createAndSubmitCb()` helper for record-submit patterns

Each simulator then layers its own API conventions on top of the shared Layer 5 compute runtime.

## Dependencies

- `@coding-adventures/compute-runtime` -- the Vulkan-inspired Layer 5 runtime
- `@coding-adventures/gpu-core` -- ISA instruction definitions

## Development

```bash
npm ci
npx vitest run --coverage
```

## License

MIT
