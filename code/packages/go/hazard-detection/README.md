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

```go
import hd "github.com/adhithyan15/coding-adventures/code/packages/go/hazard-detection"

unit := hd.NewHazardUnit(2, 1, true) // 2 ALUs, 1 FP unit, split caches

ifStage := hd.PipelineSlot{Valid: true}
idStage := hd.PipelineSlot{Valid: true, SourceRegs: []int{1}}
exStage := hd.PipelineSlot{Valid: true, DestReg: hd.IntPtr(1), DestValue: hd.IntPtr(42)}
memStage := hd.PipelineSlot{}

result := unit.Check(ifStage, idStage, exStage, memStage)
// result.Action == hd.ActionForwardFromEX
// *result.ForwardedValue == 42
```

## Priority System

FLUSH > STALL > FORWARD_FROM_EX > FORWARD_FROM_MEM > NONE
