# coding_adventures_compute_unit

**Layer 7 of the accelerator computing stack** -- the compute unit that manages
multiple parallel execution engines, schedules work across them, and provides
shared resources (memory, caches, register files).

## What is a Compute Unit?

Just as the CPU Core composes a pipeline, branch predictor, caches, and register
file into a working processor, the Compute Unit composes execution engines,
schedulers, shared memory, and caches into a working accelerator compute unit.

This layer is where the real architectural diversity shows up:

| Compute Unit | Vendor | Key Feature |
|---|---|---|
| `StreamingMultiprocessor` | NVIDIA SM | 4 warp schedulers, latency hiding via warp switching |
| `AMDComputeUnit` | AMD CU | 4 SIMD units + scalar unit, LDS |
| `MatrixMultiplyUnit` | Google TPU MXU | Systolic array, no threads, compile-time scheduling |
| `XeCore` | Intel Xe Core | 8-16 EUs with hardware threads, SLM |
| `NeuralEngineCore` | Apple ANE | MAC array + DMA, compiler-scheduled |

## Layer Position

```
Layer 11: Logic Gates
Layer 10: FP Arithmetic
Layer 9:  GPU Core (gpu-core)
Layer 8:  Parallel Execution Engine
Layer 7:  Compute Unit  <-- THIS PACKAGE
Layer 6:  Device Simulator (future)
```

## Usage

```ruby
require "coding_adventures_compute_unit"
include CodingAdventures

clock = Clock::ClockGenerator.new
sm = ComputeUnit::StreamingMultiprocessor.new(
  ComputeUnit::SMConfig.new(max_warps: 8),
  clock
)
sm.dispatch(ComputeUnit::WorkItem.new(
  work_id: 0,
  program: [GpuCore.limm(0, 2.0), GpuCore.limm(1, 3.0), GpuCore.fmul(2, 0, 1), GpuCore.halt],
  thread_count: 64
))
traces = sm.run
puts "Completed in #{traces.length} cycles, occupancy: #{sm.occupancy}"
```

## Dependencies

- `coding_adventures_parallel_execution_engine` -- WarpEngine, WavefrontEngine, SystolicArray, etc.
- `coding_adventures_gpu_core` -- GPUCore, instructions, register files
- `coding_adventures_fp_arithmetic` -- IEEE 754 floating point
- `coding_adventures_clock` -- Clock, ClockEdge

## License

MIT
