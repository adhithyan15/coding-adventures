# Branch Predictor

Simulates the branch prediction algorithms used in real CPU cores. Branch prediction is one of the most critical performance features in modern processors — without it, a deeply pipelined CPU would stall on every branch instruction, losing 10-15 cycles each time.

## What's Inside

| Module | Description |
|--------|-------------|
| `base.ts` | `BranchPredictor` interface and `Prediction` type |
| `static.ts` | `AlwaysTakenPredictor`, `AlwaysNotTakenPredictor`, `BackwardTakenForwardNotTaken` |
| `one-bit.ts` | `OneBitPredictor` — 1-bit flip-flop per branch |
| `two-bit.ts` | `TwoBitPredictor` — 2-bit saturating counter (the classic) |
| `btb.ts` | `BranchTargetBuffer` — caches branch target addresses |
| `stats.ts` | `PredictionStats` — accuracy tracking |

## How It Fits in the Stack

This package is **completely standalone** — no dependencies on other packages. It provides the branch prediction unit that plugs into a CPU core's fetch stage. Any predictor can be swapped in because they all implement the same `BranchPredictor` interface.

## Design: Pluggable Predictors

All predictors implement the same interface:

```typescript
interface BranchPredictor {
    predict(pc: number): Prediction;
    update(pc: number, taken: boolean, target?: number | null): void;
    readonly stats: PredictionStats;
    reset(): void;
}
```

This means you can swap `AlwaysTakenPredictor` for `TwoBitPredictor` without changing any other code.

## Usage Examples

### Basic prediction loop

```typescript
import { TwoBitPredictor } from "@coding-adventures/branch-predictor";

const predictor = new TwoBitPredictor(1024);

// Simulate a branch at PC=0x100
const prediction = predictor.predict(0x100);
console.log(`Predicted: ${prediction.taken ? "taken" : "not taken"}`);

// After execution, update with actual outcome
predictor.update(0x100, true);

// Check accuracy
console.log(`Accuracy: ${predictor.stats.accuracy.toFixed(1)}%`);
```

### Comparing predictors

```typescript
import {
    AlwaysTakenPredictor,
    OneBitPredictor,
    TwoBitPredictor,
} from "@coding-adventures/branch-predictor";

const predictors = {
    "Always Taken": new AlwaysTakenPredictor(),
    "1-bit": new OneBitPredictor(),
    "2-bit": new TwoBitPredictor(),
};

// Simulate a loop: 9 taken + 1 not-taken, repeated 5 times
for (const [name, pred] of Object.entries(predictors)) {
    for (let run = 0; run < 5; run++) {
        for (let i = 0; i < 10; i++) {
            pred.update(0x100, i < 9);
        }
    }
    console.log(`${name}: ${pred.stats.accuracy.toFixed(1)}%`);
}
```

### Using the BTB alongside a predictor

```typescript
import { TwoBitPredictor, BranchTargetBuffer } from "@coding-adventures/branch-predictor";

const predictor = new TwoBitPredictor();
const btb = new BranchTargetBuffer(256);

const pc = 0x100;
const prediction = predictor.predict(pc);
if (prediction.taken) {
    const target = btb.lookup(pc);  // WHERE does it go?
}

// After execution
predictor.update(pc, true, 0x200);
btb.update(pc, 0x200, "conditional");
```

## Installation

```bash
npm install @coding-adventures/branch-predictor
```

Or for development:

```bash
npm install
```

## Running Tests

```bash
npx vitest run --coverage
```
