# compute-unit

**Layer 7 of the accelerator computing stack** — compute unit simulators for five vendor architectures.

## What is a Compute Unit?

A compute unit is the organizational structure that wraps execution engines (Layer 8) with scheduling, shared memory, register files, and caches to form a complete computational building block. It's where the real architectural diversity shows up.

## Architectures

| Architecture | Class | Execution Model | Key Feature |
|---|---|---|---|
| NVIDIA SM | `StreamingMultiprocessor` | SIMT warps | Warp scheduling, occupancy |
| AMD CU | `AMDComputeUnit` | SIMD wavefronts | Scalar unit, LDS |
| Google TPU MXU | `MatrixMultiplyUnit` | Systolic dataflow | Tiling, no threads |
| Intel Xe Core | `XeCore` | SIMD + threads | EU thread arbitration |
| Apple ANE Core | `NeuralEngineCore` | Scheduled MAC | Compiler-driven, DMA |

## Usage

```python
from compute_unit import StreamingMultiprocessor, SMConfig, WorkItem
from clock import Clock
from gpu_core import limm, fmul, halt

clock = Clock(frequency_hz=1_500_000_000)
sm = StreamingMultiprocessor(SMConfig(max_warps=8), clock)

sm.dispatch(WorkItem(
    work_id=0,
    program=[limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()],
    thread_count=64,
))

traces = sm.run()
print(f"Completed in {len(traces)} cycles, occupancy: {sm.occupancy:.1%}")
```

## Dependencies

- `parallel-execution-engine` — WarpEngine, WavefrontEngine, SystolicArray, MACArrayEngine, SubsliceEngine
- `gpu-core` — GPUCore, Instruction, InstructionSet, GenericISA
- `fp-arithmetic` — FloatBits, FloatFormat, FP32/FP16/BF16
- `clock` — Clock, ClockEdge

## Layer Position

```
Layer 11: Logic Gates
    |
Layer 10: FP Arithmetic
    |
Layer 9:  GPU Core (gpu-core)
    |
Layer 8:  Parallel Execution Engine
    |
Layer 7:  Compute Unit  <-- THIS PACKAGE
    |
Layer 6:  Device Simulator (future)
```
