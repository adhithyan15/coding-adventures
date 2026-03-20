# Parallel Execution Engine

**Layer 8 of the accelerator computing stack** — parallel execution engines
that sit between individual processing elements (Layer 9, `gpu-core`) and
the compute unit (Layer 7, future `sm-simulator`).

## What's Inside

Five parallel execution engines showing how different accelerator architectures
organize parallel computation:

| Engine | Model | Architecture | Width |
|--------|-------|-------------|-------|
| `WarpEngine` | SIMT | NVIDIA CUDA / ARM Mali | 32 threads |
| `WavefrontEngine` | SIMD | AMD GCN/RDNA | 32/64 lanes |
| `SystolicArray` | Dataflow | Google TPU | NxN PEs |
| `MACArrayEngine` | Scheduled MAC | Apple ANE / NPU | M MACs |
| `SubsliceEngine` | Hybrid SIMD | Intel Xe | 8 EUs x 7 threads x SIMD8 |

## Quick Start

```python
from parallel_execution_engine import WarpEngine, WarpConfig
from clock import Clock
from gpu_core import limm, fmul, halt

clock = Clock()
engine = WarpEngine(WarpConfig(warp_width=4), clock)
engine.load_program([limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()])
traces = engine.run()
# All 4 threads computed 2.0 * 3.0 = 6.0
```

## Dependencies

- `coding-adventures-gpu-core` — processing elements (Layer 9)
- `coding-adventures-fp-arithmetic` — IEEE 754 floating-point (Layer 10)
- `coding-adventures-clock` — clock signal generation
