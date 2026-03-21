# vendor-api-simulators (Go)

Layer 3 of the accelerator computing stack: six vendor GPU API simulators implemented as thin wrappers over the [compute-runtime](../compute-runtime/) (Layer 5).

## What This Package Does

Real GPU programming requires vendor-specific APIs. This package simulates the six major GPU compute APIs so you can learn their programming models without needing actual hardware:

| Simulator | Real API | Key Paradigm |
|-----------|----------|--------------|
| **CUDA** | NVIDIA CUDA Runtime | Implicit context, stream-based, NVIDIA-only |
| **OpenCL** | Khronos OpenCL | Portable, platform/device/context model, event-based |
| **Metal** | Apple Metal | Unified memory, command encoder model, Apple-only |
| **Vulkan** | Khronos Vulkan | Ultra-explicit, verbose, maximum control |
| **WebGPU** | W3C WebGPU | Browser-safe, single queue, automatic barriers |
| **OpenGL** | Khronos OpenGL 4.3+ | Legacy state machine, integer handles |

## Architecture

```
Layer 3: vendor-api-simulators (this package)
    |
    v
Layer 5: compute-runtime
    |
    v
Layer 6: gpu-core (virtual GPU hardware)
```

Each simulator wraps the same Layer 5 compute-runtime but exposes a different API surface that matches the conventions of the real vendor API.

## Installation

```go
import vas "github.com/adhithyan15/coding-adventures/code/packages/go/vendor-api-simulators"
```

## Usage Examples

### CUDA

```go
cuda, _ := vas.NewCUDARuntime()
ptr, _ := cuda.Malloc(1024)
defer cuda.Free(ptr)

hostData := []byte{1, 2, 3, 4}
cuda.Memcpy(ptr, nil, hostData, nil, 4, vas.CUDAMemcpyHostToDevice)

kernel := vas.CUDAKernel{Name: "my_kernel"}
cuda.LaunchKernel(kernel, vas.NewDim3(4, 1, 1), vas.NewDim3(64, 1, 1),
    []*vas.CUDADevicePtr{ptr}, 0, nil)
cuda.DeviceSynchronize()
```

### OpenCL

```go
ctx, _ := vas.NewCLContext(nil)
buf, _ := ctx.CreateBuffer(vas.CLMemReadWrite, 256, nil)
prog := ctx.CreateProgramWithSource("kernel_source")
prog.Build(nil, "")
kernel, _ := prog.CreateKernel("my_kernel")
kernel.SetArg(0, buf)

queue := ctx.CreateCommandQueue(nil)
queue.EnqueueNDRangeKernel(kernel, []int{256}, []int{64}, nil)
queue.Finish()
```

### Metal

```go
device, _ := vas.NewMTLDevice()
buf, _ := device.MakeBuffer(256, vas.MTLResourceStorageModeShared)
lib := device.MakeLibrary("shader_source")
fn := lib.MakeFunction("compute_fn")
pso := device.MakeComputePipelineState(fn)

queue := device.MakeCommandQueue()
cb, _ := queue.MakeCommandBuffer()
encoder := cb.MakeComputeCommandEncoder()
encoder.SetComputePipelineState(pso)
encoder.SetBuffer(buf, 0, 0)
encoder.DispatchThreadgroups(vas.NewMTLSize(4, 1, 1), vas.NewMTLSize(64, 1, 1))
encoder.EndEncoding()
cb.Commit()
cb.WaitUntilCompleted()
```

### WebGPU

```go
device, _ := vas.NewGPUDevice("")
buf, _ := device.CreateBuffer(vas.GPUBufferDescriptor{
    Size: 256, Usage: vas.GPUBufferUsageStorage,
})
pipeline := device.CreateComputePipeline(vas.GPUComputePipelineDescriptor{Layout: "auto"})
encoder, _ := device.CreateCommandEncoder()
pass := encoder.BeginComputePass()
pass.SetPipeline(pipeline)
pass.DispatchWorkgroups(4, 1, 1)
pass.End()
cb, _ := encoder.Finish()
device.Queue.Submit([]*vas.GPUCommandBuffer{cb})
```

## Testing

```bash
go test -v ./...
```

Coverage target: 85%+. Current: 87.4% with 237 tests across all six simulators plus cross-API integration tests.

## Package Name

```go
package vendorapisimulators
```
