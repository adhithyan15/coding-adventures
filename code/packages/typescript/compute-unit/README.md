# @coding-adventures/compute-unit

**Layer 7 of the accelerator computing stack** -- compute unit simulators for five vendor architectures.

## What is a Compute Unit?

A compute unit is the organizational structure that wraps execution engines (Layer 8) with scheduling, shared memory, register files, and caches to form a complete computational building block.

```
Layer 9:  gpu-core (one core, one instruction at a time)
    |
Layer 8:  parallel-execution-engine (warps, wavefronts, systolic arrays)
    |
Layer 7:  compute-unit (THIS PACKAGE)
    |
    +-- StreamingMultiprocessor  -- NVIDIA SM
    +-- AMDComputeUnit           -- AMD CU (GCN/RDNA)
    +-- MatrixMultiplyUnit       -- Google TPU MXU
    +-- XeCore                   -- Intel Xe Core
    +-- NeuralEngineCore         -- Apple ANE Core
```

## Usage

### NVIDIA Streaming Multiprocessor

```typescript
import { StreamingMultiprocessor, makeSMConfig, makeWorkItem } from "@coding-adventures/compute-unit";
import { limm, fmul, halt } from "@coding-adventures/gpu-core";

const sm = new StreamingMultiprocessor(makeSMConfig({ maxWarps: 8 }));
sm.dispatch(makeWorkItem({
  workId: 0,
  program: [limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()],
  threadCount: 64,
}));
const traces = sm.run();
console.log(`Completed in ${traces.length} cycles, occupancy: ${(sm.occupancy * 100).toFixed(1)}%`);
```

### Google TPU MXU

```typescript
import { MatrixMultiplyUnit, makeMXUConfig } from "@coding-adventures/compute-unit";

const mxu = new MatrixMultiplyUnit(makeMXUConfig({ arrayRows: 4, arrayCols: 4 }));
const result = mxu.runMatmul(
  [[1, 2], [3, 4]],
  [[5, 6], [7, 8]],
  "relu",
);
// result = [[19, 22], [43, 50]]
```

### Apple Neural Engine

```typescript
import { NeuralEngineCore, makeANECoreConfig } from "@coding-adventures/compute-unit";

const ane = new NeuralEngineCore(makeANECoreConfig({ numMacs: 16 }));
const result = ane.runInference(
  [[1, 2, 3, 4]],
  [[0.5], [0.5], [0.5], [0.5]],
  "relu",
);
// result = [[5.0]]
```

## Dependencies

- `@coding-adventures/parallel-execution-engine` -- WarpEngine, WavefrontEngine, SystolicArray, MACArrayEngine, SubsliceEngine
- `@coding-adventures/gpu-core` -- Instruction, InstructionSet, GenericISA, opcodes
- `@coding-adventures/fp-arithmetic` -- FloatFormat, FP32, FP16, BF16

## Testing

```bash
npm test
```
