# cpu_pipeline

Configurable N-stage CPU instruction pipeline simulator in Elixir.

## Overview

This package implements the central execution engine of a CPU core: the instruction pipeline. Instead of completing one instruction before starting the next, a pipelined CPU overlaps instruction execution for higher throughput.

The pipeline is ISA-independent. It manages the flow of "tokens" (representing instructions) through configurable stages, handling stalls, flushes, and forwarding. The actual work of each stage is performed by callback functions (lambdas) injected from the CPU core.

Features:

- **Configurable depth**: 2-stage minimal to 20+ stage deep pipelines
- **Pipeline tokens**: ISA-independent instruction representation using structs
- **Stall support**: freeze earlier stages and insert bubbles (load-use hazards)
- **Flush support**: replace speculative stages with bubbles (branch misprediction)
- **Forwarding integration**: callback-based forwarding path activation
- **Branch predictor integration**: callback-based next-PC prediction
- **Snapshots and traces**: capture pipeline state at every cycle for visualization
- **Statistics**: IPC, CPI, stall cycles, flush cycles, bubble cycles
- **Configuration presets**: `classic_5_stage/0` and `deep_13_stage/0`
- **Functional design**: immutable state, no mutation -- each step returns a new pipeline

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

```elixir
alias CodingAdventures.CpuPipeline.Pipeline

# Create a classic 5-stage pipeline.
config = Pipeline.classic_5_stage()

{:ok, pipeline} = Pipeline.new(
  config,
  fetch_callback,      # fn pc -> raw_instruction_bits end
  decode_callback,     # fn raw, token -> decoded_token end
  execute_callback,    # fn token -> token_with_alu_result end
  memory_callback,     # fn token -> token_with_memory_data end
  writeback_callback   # fn token -> :ok end
)

# Optionally add hazard detection and branch prediction.
pipeline = Pipeline.set_hazard_func(pipeline, my_hazard_detector)
pipeline = Pipeline.set_predict_func(pipeline, my_branch_predictor)

# Step one cycle at a time.
{pipeline, snapshot} = Pipeline.step(pipeline)

# Or run until halt or max cycles.
{pipeline, stats} = Pipeline.run(pipeline, 10000)
```

## Configuration Presets

| Preset | Stages | Inspired By |
|--------|--------|-------------|
| `classic_5_stage/0` | IF, ID, EX, MEM, WB | MIPS R2000 (1985) |
| `deep_13_stage/0` | IF1-IF3, ID1-ID3, EX1-EX3, MEM1-MEM3, WB | ARM Cortex-A78 (2020) |

Custom configurations are also supported by building your own `PipelineConfig` struct.

## Dependencies

This package uses dependency injection (callbacks) and has no Elixir package dependencies.

## Testing

```bash
mix test --cover
```
