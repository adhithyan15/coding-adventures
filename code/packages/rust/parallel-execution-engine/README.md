# parallel-execution-engine (Rust)

**Layer 8 of the accelerator computing stack** -- parallel execution engines
that orchestrate thousands of processing elements in parallel.

## What is this?

Layer 9 (`gpu-core`) gave us a single processing element. Layer 8 takes many
of those elements and orchestrates them to execute together. But *how* they're
orchestrated differs fundamentally across architectures.

This crate provides five parallel execution engines, each implementing a
different execution model:

| Engine            | Model         | Real Hardware  | Key Feature              |
|-------------------|---------------|----------------|--------------------------|
| `WarpEngine`      | SIMT          | NVIDIA/ARM     | Hardware divergence mgmt |
| `WavefrontEngine` | SIMD          | AMD GCN/RDNA   | Explicit EXEC mask       |
| `SubsliceEngine`  | SIMD+MT       | Intel Xe       | Thread arbitration       |
| `SystolicArray`   | Dataflow      | Google TPU     | No instructions at all   |
| `MACArrayEngine`  | Scheduled MAC | Apple ANE      | Compiler-driven schedule |

## Layer Position

```
Layer 11: Logic Gates
    |
Layer 10: FP Arithmetic
    |
Layer 9:  GPU Core (one core, one instruction at a time)
    |
Layer 8:  Parallel Execution Engine  <-- THIS PACKAGE
    |
Layer 7:  Compute Unit (future)
```

## Usage

```rust
use gpu_core::opcodes::{limm, fmul, halt};
use parallel_execution_engine::warp_engine::{WarpEngine, WarpConfig};
use parallel_execution_engine::protocols::ParallelExecutionEngine;

// Create a 32-thread SIMT warp (NVIDIA style)
let mut engine = WarpEngine::new(WarpConfig::default());

// Load a program that all threads execute
engine.load_program(vec![
    limm(0, 2.0),
    limm(1, 3.0),
    fmul(2, 0, 1),
    halt(),
]);

// Give each thread different data
for t in 0..32 {
    engine.set_thread_register(t, 0, t as f64);
}

// Run all threads
let traces = engine.run(1000).unwrap();

// Read per-thread results
println!("Thread 0: R2 = {}", engine.threads()[0].core.registers.read_float(2));
```

## Dependencies

- `gpu-core` -- provides the `GPUCore` processing element used internally
- `fp-arithmetic` -- IEEE 754 floating-point operations
