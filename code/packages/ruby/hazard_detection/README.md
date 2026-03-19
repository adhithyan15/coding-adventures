# coding_adventures_hazard_detection

Pipeline hazard detection for a classic 5-stage CPU (IF -> ID -> EX -> MEM -> WB).

## What It Does

Detects three types of pipeline hazards and determines the correct action:

- **Data hazards (RAW)**: When an instruction reads a register that a previous instruction hasn't finished writing. Resolved by forwarding from EX or MEM stages, or stalling for load-use hazards.
- **Control hazards**: When a branch is mispredicted and the wrong instructions are in the pipeline. Resolved by flushing IF and ID stages.
- **Structural hazards**: When two instructions need the same hardware resource (ALU, FP unit, memory port). Resolved by stalling.

## How It Fits

This sits between the pipeline and the instruction decoder. The decoder fills `PipelineSlot` structs with register usage and resource requirements; the hazard unit decides whether to forward, stall, or flush.

## Usage

```ruby
require "coding_adventures_hazard_detection"

include CodingAdventures::HazardDetection

unit = HazardUnit.new(num_alus: 2, split_caches: true)

if_stage = PipelineSlot.new(valid: true)
id_stage = PipelineSlot.new(valid: true, source_regs: [1], uses_alu: false)
ex_stage = PipelineSlot.new(valid: true, dest_reg: 1, dest_value: 42, uses_alu: false)
mem_stage = PipelineSlot.new(valid: false)

result = unit.check(if_stage, id_stage, ex_stage, mem_stage)
# result.action => :forward_ex
# result.forwarded_value => 42
```

## Priority System

FLUSH > STALL > FORWARD_FROM_EX > FORWARD_FROM_MEM > NONE
