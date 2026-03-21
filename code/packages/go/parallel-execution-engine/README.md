# parallel-execution-engine (Go)

Layer 8 of the accelerator computing stack -- the parallel execution engine
that sits between individual processing elements (Layer 9, `gpu-core`) and
the compute unit (Layer 7, future `sm-simulator`).

## What is this?

This package implements **five different parallel execution engines**, each
modeling a real-world accelerator architecture:

| Engine | Model | Real Hardware | Width |
|---|---|---|---|
| `WarpEngine` | SIMT | NVIDIA CUDA, ARM Mali | 32 threads |
| `WavefrontEngine` | SIMD | AMD GCN/RDNA | 32/64 lanes |
| `SystolicArray` | Dataflow | Google TPU | NxN PEs |
| `MACArrayEngine` | Scheduled MAC | Apple ANE, Qualcomm | N MACs |
| `SubsliceEngine` | Hybrid SIMD | Intel Arc Xe | EUs x threads x SIMD8 |

All five engines implement the `ParallelExecutionEngine` interface, allowing
higher layers to drive any engine uniformly.

## Layer Position

```
Layer 11: Logic Gates
Layer 10: FP Arithmetic
Layer 9:  GPU Core (gpu-core) -- one core, one instruction at a time
Layer 8:  Parallel Execution Engine <-- THIS PACKAGE
Layer 7:  Compute Unit (future)
```

## Usage

```go
import (
    pe "github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine"
    "github.com/adhithyan15/coding-adventures/code/packages/go/clock"
    gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// Create a 4-thread warp engine
clk := clock.New(1000000)
engine := pe.NewWarpEngine(pe.DefaultWarpConfig(), clk)

// Load a program
engine.LoadProgram([]gpucore.Instruction{
    gpucore.Limm(0, 2.0),
    gpucore.Limm(1, 3.0),
    gpucore.Fmul(2, 0, 1),
    gpucore.Halt(),
})

// Run and get traces
traces, err := engine.Run(10000)
```

## Dependencies

- `gpu-core` -- processing elements (Layer 9)
- `fp-arithmetic` -- IEEE 754 floating-point operations (Layer 10)
- `clock` -- clock signal generation

## Testing

```bash
go test ./... -v -cover
```
