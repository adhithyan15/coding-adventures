# hazard-detection

Pipeline hazard detection for a classic 5-stage CPU (IF -> ID -> EX -> MEM -> WB).

## What It Does

Detects three types of pipeline hazards and determines the correct action:

- **Data hazards (RAW)**: When an instruction reads a register that a previous instruction hasn't finished writing. Resolved by forwarding from EX or MEM stages, or stalling for load-use hazards.
- **Control hazards**: When a branch is mispredicted and the wrong instructions are in the pipeline. Resolved by flushing IF and ID stages.
- **Structural hazards**: When two instructions need the same hardware resource (ALU, FP unit, memory port). Resolved by stalling.

## How It Fits

This sits between the pipeline and the instruction decoder. The decoder fills `PipelineSlot` structs with register usage and resource requirements; the hazard unit decides whether to forward, stall, or flush.

## Usage

```rust
use hazard_detection::types::{PipelineSlot, HazardAction};
use hazard_detection::hazard_unit::HazardUnit;

let mut unit = HazardUnit::new(2, 1, true); // 2 ALUs, 1 FP unit, split caches

let if_stage = PipelineSlot { valid: true, ..Default::default() };
let id_stage = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
let ex_stage = PipelineSlot { valid: true, dest_reg: Some(1), dest_value: Some(42), ..Default::default() };
let mem_stage = PipelineSlot::default();

let result = unit.check(&if_stage, &id_stage, &ex_stage, &mem_stage);
assert_eq!(result.action, HazardAction::ForwardFromEX);
assert_eq!(result.forwarded_value, Some(42));
```

## Priority System

FLUSH > STALL > FORWARD_FROM_EX > FORWARD_FROM_MEM > NONE
