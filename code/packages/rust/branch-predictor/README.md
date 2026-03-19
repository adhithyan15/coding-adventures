# branch-predictor

Pluggable branch prediction algorithms for CPU simulation in Rust.

## What it does

Implements several branch prediction strategies used in real and simulated CPUs, from trivial static predictors to the classic two-bit saturating counter. Also includes a Branch Target Buffer (BTB) for caching branch target addresses.

## How it fits in the stack

This is the Rust port of the Python `branch-predictor` package. It sits at layer 9 of the accelerator stack, working alongside the cache hierarchy and hazard detection unit inside the CPU pipeline. The branch predictor sits in the fetch stage and guesses whether branches are taken, enabling speculative execution.

## Key types

- `Prediction` -- the predictor's guess (taken, confidence, target)
- `BranchPredictor` -- trait that all predictors implement (predict, update, stats, reset)
- `AlwaysTakenPredictor` -- static, ~60% accurate
- `AlwaysNotTakenPredictor` -- static baseline, ~30-40% accurate
- `BackwardTakenForwardNotTaken` -- direction heuristic, ~65-75% accurate
- `OneBitPredictor` -- 1-bit per branch, learns from history
- `TwoBitPredictor` -- 2-bit saturating counter with hysteresis (the textbook classic)
- `TwoBitState` -- the four states of the saturating counter
- `BranchTargetBuffer` -- caches where branches go (separate from direction prediction)
- `PredictionStats` -- accuracy and misprediction rate tracking

## Usage

```rust
use branch_predictor::{TwoBitPredictor, TwoBitState, BranchPredictor};

let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);

// Simulate a loop: 9 taken, 1 not-taken
for i in 0..10 {
    pred.predict(0x100);
    pred.update(0x100, i < 9, Some(0x50));
}

println!("Accuracy: {:.1}%", pred.stats().accuracy());
```

## Running tests

```bash
cargo test -p branch-predictor
```
