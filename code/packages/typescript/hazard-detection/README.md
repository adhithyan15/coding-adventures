# Hazard Detection

Pipeline hazard detection and resolution for a pipelined CPU. This package detects the three types of hazards that can occur when multiple instructions are in flight simultaneously:

1. **Data hazards (RAW)** - An instruction reads a register that a previous instruction hasn't finished writing yet. Resolved by forwarding (bypassing) values from later pipeline stages, or stalling when forwarding isn't possible (load-use hazard).

2. **Control hazards** - A branch was mispredicted, so the pipeline fetched wrong instructions. Resolved by flushing the pipeline and restarting from the correct address.

3. **Structural hazards** - Two instructions need the same hardware resource (ALU, FP unit, memory port) at the same time. Resolved by stalling one instruction.

## How It Fits in the Stack

This package sits between the pipeline and the instruction decoder. It receives `PipelineSlot` descriptors (ISA-independent) from each pipeline stage and returns a `HazardResult` telling the pipeline what to do (forward, stall, flush, or proceed normally).

```
┌────────────┐    ┌────────────────┐    ┌──────────┐
│  Pipeline   │───>│ Hazard Unit    │───>│ Pipeline  │
│  Stages     │    │ (this package) │    │ Control   │
└────────────┘    └────────────────┘    └──────────┘
```

The package is standalone - it works with any pipeline implementation and any ISA.

## Usage

```typescript
import { HazardUnit, PipelineSlot, HazardAction } from "@coding-adventures/hazard-detection";

// Create the hazard unit (configurable hardware resources)
const unit = new HazardUnit({ numAlus: 1, numFpUnits: 1, splitCaches: true });

// Each cycle, describe what's in each pipeline stage
const ifStage = new PipelineSlot({ valid: true, pc: 0x100C });
const idStage = new PipelineSlot({ valid: true, pc: 0x1008, sourceRegs: [1, 2], usesAlu: true });
const exStage = new PipelineSlot({ valid: true, pc: 0x1004, destReg: 1, destValue: 42, usesAlu: true });
const memStage = new PipelineSlot({ valid: true, pc: 0x1000, destReg: 3, usesAlu: true });

// Check for hazards
const result = unit.check(ifStage, idStage, exStage, memStage);

if (result.action === HazardAction.FORWARD_FROM_EX) {
    console.log(`Forward value ${result.forwardedValue} from EX stage`);
} else if (result.action === HazardAction.STALL) {
    console.log(`Stall for ${result.stallCycles} cycle(s): ${result.reason}`);
} else if (result.action === HazardAction.FLUSH) {
    console.log(`Flush ${result.flushCount} stages: ${result.reason}`);
}

// Performance stats
console.log(`Total stalls: ${unit.stallCount}`);
console.log(`Total flushes: ${unit.flushCount}`);
console.log(`Total forwards: ${unit.forwardCount}`);
```

## Individual Detectors

You can also use the detectors independently:

```typescript
import { DataHazardDetector, ControlHazardDetector, StructuralHazardDetector } from "@coding-adventures/hazard-detection";

// Data hazard detection only
const dataDetector = new DataHazardDetector();
const result1 = dataDetector.detect(idStage, exStage, memStage);

// Control hazard detection only
const controlDetector = new ControlHazardDetector();
const result2 = controlDetector.detect(exStage);

// Structural hazard detection only
const structuralDetector = new StructuralHazardDetector({ numAlus: 2, splitCaches: true });
const result3 = structuralDetector.detect(idStage, exStage);
```

## Priority System

When multiple hazards occur simultaneously, the highest-priority action wins:

| Priority | Action | Meaning |
|----------|--------|---------|
| 4 (highest) | FLUSH | Branch misprediction - wrong instructions in pipeline |
| 3 | STALL | Data not ready - must wait |
| 2 | FORWARD_FROM_EX | Forward value from EX stage |
| 1 | FORWARD_FROM_MEM | Forward value from MEM stage |
| 0 (lowest) | NONE | All clear, proceed normally |

## Installation

```bash
npm install @coding-adventures/hazard-detection
```

## Development

```bash
npm install
npx vitest run --coverage
```
