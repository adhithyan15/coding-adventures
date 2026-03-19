# Branch Predictor (Go)

Branch prediction simulators built from first principles -- a Go port of the Python `branch-predictor` package.

## What It Does

In CPU design, the branch predictor sits at the front of the pipeline and guesses whether each branch instruction will be taken or not. This package implements several predictor strategies as educational simulators:

- **AlwaysTakenPredictor** -- static, always predicts "taken" (~60-70% accurate)
- **AlwaysNotTakenPredictor** -- static, always predicts "not taken" (~30-40%)
- **BTFNTPredictor** -- static direction heuristic (~65-75%)
- **OneBitPredictor** -- dynamic, 1-bit per branch (learns last outcome)
- **TwoBitPredictor** -- dynamic, 2-bit saturating counter (classic textbook)
- **BranchTargetBuffer** -- caches WHERE branches go (target addresses)

## How It Fits in the Stack

This is a standalone package with no dependencies on other coding-adventures Go packages. It models the branch prediction unit that sits in the fetch stage of the CPU pipeline.

## Usage

```go
package main

import (
    bp "github.com/adhithyan15/coding-adventures/code/packages/go/branch-predictor"
)

func main() {
    predictor := bp.NewTwoBitPredictor(1024, bp.WeaklyNotTaken)

    // Simulate a branch at PC 0x100
    pred := predictor.Predict(0x100)
    fmt.Println(pred.Taken) // false (cold start)

    // Feed back the actual outcome
    predictor.Update(0x100, true, bp.NoTarget)

    // Now it predicts taken
    pred = predictor.Predict(0x100)
    fmt.Println(pred.Taken) // true

    // Check accuracy
    fmt.Printf("Accuracy: %.1f%%\n", predictor.Stats().Accuracy())
}
```

## Running Tests

```bash
go test ./... -v -cover
```
