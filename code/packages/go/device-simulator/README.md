# device-simulator (Go)

Layer 6 of the accelerator computing stack — complete device simulators that model entire accelerator chips with all their compute units, global memory, caches, and work distributors.

## What is a Device Simulator?

A device simulator models a **complete accelerator** — not just one compute unit, but the entire chip. Think of it as the difference between simulating one factory floor (Layer 7) versus simulating the entire factory complex:

| Layer | Scope | Analogy |
|-------|-------|---------|
| Layer 7 (Compute Unit) | One SM / CU / MXU | One factory floor |
| **Layer 6 (Device)** | **The whole chip** | **Entire factory complex** |

The device layer adds four new concepts:

1. **Global Memory (VRAM)** — the large device-wide memory (16-80 GB). All compute units share it.
2. **L2 Cache** — sits between compute units and global memory, reducing average latency.
3. **Work Distributor** — takes kernel launches and assigns thread blocks to available compute units.
4. **Host Interface** — the connection to the CPU (PCIe, NVLink, or unified memory).

## Supported Architectures

| Device | Architecture | Work Distribution | Memory |
|--------|-------------|-------------------|--------|
| **NVIDIA GPU** | Streaming Multiprocessors | GigaThread Engine (round-robin) | HBM3 + L2 Cache |
| **AMD GPU** | Compute Units in Shader Engines | Command Processor | GDDR6 + Infinity Cache |
| **Google TPU** | MXU (systolic array) | Scalar→MXU→Vector pipeline | HBM2e |
| **Intel GPU** | Xe-Cores in Xe-Slices | Command Streamer | GDDR6 + L2 Cache |
| **Apple ANE** | Neural Engine Cores | Compiler-generated schedule | Unified Memory (zero-copy) |

## Usage

### NVIDIA GPU

```go
import ds "github.com/adhithyan15/coding-adventures/code/packages/go/device-simulator"

gpu := ds.NewNvidiaGPU(nil, 4) // 4 SMs

// Allocate and copy data
addr, _ := gpu.Malloc(1024)
gpu.MemcpyHostToDevice(addr, data)

// Launch kernel
gpu.LaunchKernel(ds.KernelDescriptor{
    Name:    "saxpy",
    Program: program,
    GridDim: [3]int{4, 1, 1},
    BlockDim: [3]int{32, 1, 1},
})

// Run to completion
traces := gpu.Run(10000)
```

### Google TPU (dataflow)

```go
tpu := ds.NewGoogleTPU(nil, 128) // 128x128 MXU

tpu.LaunchKernel(ds.KernelDescriptor{
    Operation:  "matmul",
    InputData:  inputMatrix,
    WeightData: weightMatrix,
})

traces := tpu.Run(5000)
```

### Apple ANE (unified memory)

```go
ane := ds.NewAppleANE(nil, 4) // 4 NE cores

// Zero-copy: no PCIe transfer overhead
cycles, _ := ane.MemcpyHostToDevice(addr, data) // cycles == 0

ane.LaunchKernel(ds.KernelDescriptor{
    Operation:  "conv2d",
    InputData:  input,
    WeightData: weights,
})

traces := ane.Run(5000)
```

## Interface

All five device types implement the `AcceleratorDevice` interface:

```go
type AcceleratorDevice interface {
    Name() string
    Config() DeviceConfig
    Malloc(size int) (int, error)
    Free(address int)
    MemcpyHostToDevice(dst int, data []byte) (int, error)
    MemcpyDeviceToHost(src int, size int) ([]byte, int, error)
    LaunchKernel(kernel KernelDescriptor)
    Step(edge clock.ClockEdge) DeviceTrace
    Run(maxCycles int) []DeviceTrace
    Idle() bool
    Reset()
    Stats() DeviceStats
    ComputeUnits() []computeunit.ComputeUnit
    GlobalMem() *SimpleGlobalMemory
}
```

## Dependencies

- `cache` — L2/Infinity Cache simulation
- `clock` — clock signal generation
- `compute-unit` — SM, CU, MXU, XeCore, ANECore implementations
- `gpu-core` — instruction set and ISA definitions
- `fp-arithmetic` — floating-point format definitions
- `parallel-execution-engine` — warp/wavefront/systolic engines

## Testing

```bash
go test ./... -v -cover
```

Coverage target: 95%+
