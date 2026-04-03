# coding-adventures-hazard-detection (Lua)

Pipeline hazard detection for pipelined CPUs — data hazards, control hazards,
and structural hazards. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What Are Pipeline Hazards?

When a CPU pipeline overlaps multiple instructions, conflicts arise:

**Data Hazards (RAW — Read After Write):**
```
ADD R1, R2, R3   ; writes R1 in WB (cycle 5)
SUB R4, R1, R5   ; reads R1 in ID (cycle 3) — WRONG VALUE!
```
Solution: forward the result from EX/MEM, or stall if forwarding is too late.

**Control Hazards (branch misprediction):**
```
BEQ R1, R2, target  ; branch resolved in EX (cycle 3)
ADD R3, R4, R5      ; already fetched — must FLUSH if branch taken!
```

**Structural Hazards (resource conflict):**
```
IF stage + MEM stage both need memory in the same cycle (unified cache)
```

## Package Contents

| Type | Description |
|------|-------------|
| `PipelineSlot` | Snapshot of one pipeline stage |
| `HazardResult` | Detection result: action + forwarded value + reason |
| `DataHazardDetector` | Detects RAW hazards; returns forward/stall action |
| `ControlHazardDetector` | Detects branch mispredictions; returns flush action |
| `StructuralHazardDetector` | Detects resource conflicts |

## Usage

```lua
local hd   = require("coding_adventures.hazard_detection")
local Slot = hd.PipelineSlot
local Data = hd.DataHazardDetector

local det = Data.new()

-- ADD R1, ... is in EX stage (will write R1 = 42)
local ex_slot = Slot.new({ valid=true, dest_reg=1, dest_value=42 })

-- SUB ..., R1, ... is in ID stage (reads R1)
local id_slot = Slot.new({ valid=true, source_regs={1} })

local result = det:detect(id_slot, ex_slot, Slot.empty())
print(result.action)           -- "forward_ex"
print(result.forwarded_value)  -- 42
print(result.reason)           -- "RAW hazard on R1: forwarding ..."
```

## Layer Position

This package is **D03** in the cpu architecture stack:

```
Core (D05)
└── Pipeline (D04)
      └── Hazard Detection (D03) ← this package
```

It has no dependencies on other custom packages — it is pure combinational
logic operating on pipeline slot snapshots.
