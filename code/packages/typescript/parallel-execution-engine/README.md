# @coding-adventures/parallel-execution-engine

**Layer 8 of the accelerator computing stack** -- the parallel execution engine
that sits between individual processing elements (Layer 9, `gpu-core`) and the
compute unit (Layer 7, future `sm-simulator`).

This is where parallelism happens. Layer 9 gave us a single core that executes
one instruction at a time. Layer 8 takes many of those cores and orchestrates
them to execute in parallel -- but the *way* they're orchestrated differs
fundamentally across architectures.

## Five Execution Models

| Engine | Model | Architecture | Key Concept |
|--------|-------|-------------|-------------|
| `WarpEngine` | SIMT | NVIDIA / ARM Mali | 32 threads, divergence stack |
| `WavefrontEngine` | SIMD | AMD GCN/RDNA | Wide vector ALU, EXEC mask |
| `SystolicArray` | Dataflow | Google TPU | NxN PE grid, no instructions |
| `MACArrayEngine` | Scheduled MAC | Apple ANE / NPU | Compiler-driven schedule |
| `SubsliceEngine` | Hybrid SIMD | Intel Xe | Multi-threaded SIMD8 EUs |

## Installation

```bash
npm install @coding-adventures/parallel-execution-engine
```

## Usage

### SIMT (NVIDIA-style warp)

```typescript
import { WarpEngine, makeWarpConfig } from "@coding-adventures/parallel-execution-engine";
import { limm, fmul, halt } from "@coding-adventures/gpu-core";

const engine = new WarpEngine(makeWarpConfig({ warpWidth: 4 }));
engine.loadProgram([limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]);
const traces = engine.run();
engine.threads[0].core.registers.readFloat(2);  // 6.0
```

### Systolic Array (TPU-style matmul)

```typescript
import { SystolicArray, makeSystolicConfig } from "@coding-adventures/parallel-execution-engine";

const array = new SystolicArray(makeSystolicConfig({ rows: 2, cols: 2 }));
const result = array.runMatmul(
  [[1, 2], [3, 4]],  // activations
  [[5, 6], [7, 8]],  // weights
);
// result = [[19, 22], [43, 50]]
```

### MAC Array (NPU-style scheduled)

```typescript
import {
  MACArrayEngine, makeMACArrayConfig,
  makeMACScheduleEntry, MACOperation,
} from "@coding-adventures/parallel-execution-engine";

const engine = new MACArrayEngine(makeMACArrayConfig({ numMacs: 4 }));
engine.loadInputs([1, 2, 3, 4]);
engine.loadWeights([1, 1, 1, 1]);
engine.loadSchedule([
  makeMACScheduleEntry({ cycle: 1, operation: MACOperation.MAC, inputIndices: [0,1,2,3], weightIndices: [0,1,2,3] }),
  makeMACScheduleEntry({ cycle: 2, operation: MACOperation.REDUCE, outputIndex: 0 }),
]);
engine.run();
engine.readOutputs()[0];  // 10.0
```

## Dependencies

- `@coding-adventures/gpu-core` -- GPUCore processing elements
- `@coding-adventures/fp-arithmetic` -- IEEE 754 floating-point operations

## License

MIT
