# cpu-pipeline

Configurable N-stage CPU instruction pipeline simulator in Go.

## Overview

This package implements the central execution engine of a CPU core: the instruction pipeline. Instead of completing one instruction before starting the next, a pipelined CPU overlaps instruction execution for higher throughput.

The pipeline is ISA-independent. It manages the flow of "tokens" (representing instructions) through configurable stages, handling stalls, flushes, and forwarding. The actual work of each stage is performed by callback functions injected from the CPU core.

Features:

- **Configurable depth**: 2-stage minimal to 20+ stage deep pipelines
- **Pipeline tokens**: ISA-independent instruction representation
- **Stall support**: freeze earlier stages and insert bubbles (load-use hazards)
- **Flush support**: replace speculative stages with bubbles (branch misprediction)
- **Forwarding integration**: callback-based forwarding path activation
- **Branch predictor integration**: callback-based next-PC prediction
- **Snapshots and traces**: capture pipeline state at every cycle for visualization
- **Statistics**: IPC, CPI, stall cycles, flush cycles, bubble cycles
- **Configuration presets**: `Classic5Stage()` and `Deep13Stage()`

## Layer Position

```
Core (D05)
  Pipeline (D04) <- THIS PACKAGE
    IF -> ID -> EX -> MEM -> WB  (classic 5-stage)
    Branch Predictor (D02) <- provides predictions to IF stage
    Hazard Detection (D03) <- stall/flush signals for pipeline control
    Forwarding Unit (D03)  <- bypass paths between stages
    Cache (D01)            <- IF reads L1I, MEM reads/writes L1D
```

## Usage

```go
import cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"

// Create a classic 5-stage pipeline.
config := cpupipeline.Classic5Stage()

pipeline, err := cpupipeline.NewPipeline(
    config,
    fetchCallback,      // (pc int) -> raw instruction bits
    decodeCallback,     // (raw int, token) -> decoded token
    executeCallback,    // (token) -> token with ALU result
    memoryCallback,     // (token) -> token with memory data
    writebackCallback,  // (token) -> void
)

// Optionally add hazard detection and branch prediction.
pipeline.SetHazardFunc(myHazardDetector)
pipeline.SetPredictFunc(myBranchPredictor)

// Step one cycle at a time.
snapshot := pipeline.Step()

// Or run until halt or max cycles.
stats := pipeline.Run(10000)
fmt.Printf("IPC: %.3f, Stalls: %d, Flushes: %d\n",
    stats.IPC(), stats.StallCycles, stats.FlushCycles)
```

## Configuration Presets

| Preset | Stages | Inspired By |
|--------|--------|-------------|
| `Classic5Stage()` | IF, ID, EX, MEM, WB | MIPS R2000 (1985) |
| `Deep13Stage()` | IF1-IF3, ID1-ID3, EX1-EX3, MEM1-MEM3, WB | ARM Cortex-A78 (2020) |

Custom configurations are also supported by providing your own `PipelineConfig`.

## Pipeline Behavior

### Normal Flow

```
Cycle:  1    2    3    4    5    6    7
IF:    I1   I2   I3   I4   I5   I6   I7
ID:    --   I1   I2   I3   I4   I5   I6
EX:    --   --   I1   I2   I3   I4   I5
MEM:   --   --   --   I1   I2   I3   I4
WB:    --   --   --   --   I1   I2   I3
                           ^1st  ^2nd ^3rd completion
```

### Stall (Load-Use Hazard)

```
Cycle:  3    4(stall)  5
IF:    I3   I3(frozen) I3
ID:    I2   I2(frozen) I2
EX:    I1   ---bubble  I2
MEM:   --   I1         ---
WB:    --   --         I1
```

### Flush (Branch Misprediction)

```
Cycle:  3    4(flush)  5
IF:    I3   new_I1     new_I2
ID:    I2   ---bubble  new_I1
EX:    BR   BR         ---
MEM:   --   --         BR
WB:    --   --         --
```

## Dependencies

This package uses dependency injection (callbacks) and has no Go package dependencies. It is compatible with:

- `cache` (D01): fetch callback reads from L1I, memory callback reads/writes L1D
- `branch-predictor` (D02): predict callback provides speculative next-PC
- `hazard-detection` (D03): hazard callback provides stall/flush/forward signals
- `clock` (D01): can be driven by Clock.Tick() for cycle-accurate simulation

## Testing

```bash
go test ./... -v -cover
```

Coverage: 98.5% of statements.
