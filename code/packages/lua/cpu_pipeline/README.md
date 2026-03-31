# coding-adventures-cpu-pipeline (Lua)

A configurable N-stage CPU instruction pipeline simulator. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What is a CPU Pipeline?

A CPU pipeline allows a processor to overlap the execution of multiple
instructions — while one instruction executes, the next is decoded, and the one
after is fetched. This is the same principle as a factory assembly line: each
workstation handles one task, then passes the work downstream.

```
5-Stage Pipeline (classic RISC):

Cycle:  1    2    3    4    5    6    7    8    9
Inst1: [IF] [ID] [EX] [ME] [WB]
Inst2:      [IF] [ID] [EX] [ME] [WB]
Inst3:           [IF] [ID] [EX] [ME] [WB]
```

After the pipeline fills, one instruction completes every cycle.

## Package Contents

| Type | Description |
|------|-------------|
| `Pipeline` | The configurable pipeline engine |
| `PipelineConfig` | Stage configuration (with validation) |
| `PipelineStage` | Individual stage definition |
| `Token` | Unit of work flowing through the pipeline |
| `HazardResponse` | Control signals from hazard detector |
| `PipelineStats` | IPC, CPI, stall/flush counters |
| `Snapshot` | Point-in-time pipeline state |

## Usage

```lua
local cpu_pipeline   = require("coding_adventures.cpu_pipeline")
local Pipeline       = cpu_pipeline.Pipeline
local PipelineConfig = cpu_pipeline.PipelineConfig

-- Callback functions
local memory = { [0]=0x00000001, [4]=0x00000002 }  -- simple memory
local function fetch(pc)    return memory[pc] or 0 end
local function decode(r, t) t.opcode = "NOP"; return t end
local function execute(t)   return t end
local function mem_cb(t)    return t end
local function writeback(t) end

-- Create a 5-stage pipeline
local result = Pipeline.new(
    PipelineConfig.classic_5_stage(),
    fetch, decode, execute, mem_cb, writeback
)
assert(result.ok, result.err)
local p = result.pipeline

-- Run 10 cycles
local stats = p:run(10)
print(string.format("IPC: %.3f", stats:ipc()))  -- IPC: 0.600 (6/10)
```

## How it Fits in the Stack

```
Core (D05) — integrates pipeline with caches, register file, memory controller
  └── Pipeline (D04) — this package
        ├── Hazard Detection (D03) — detects RAW hazards, stalls, flushes
        ├── Branch Predictor (D02) — predicts branch targets
        └── Cache (D01) — IF reads L1I, MEM reads/writes L1D
```

## Hazard Detection

The pipeline accepts an optional `hazard_fn` callback that returns a
`HazardResponse` each cycle:

```lua
p:set_hazard_fn(function(stages)
    -- stages[1] = IF stage token, stages[2] = ID, etc.
    local HazardResponse = cpu_pipeline.HazardResponse
    -- Example: stall when ID stage has a load-use dependency
    return HazardResponse.new({ action = "none" })
end)
```

Available actions: `"none"`, `"stall"`, `"flush"`, `"forward_from_ex"`,
`"forward_from_mem"`.

## Layer Position

This package is **D04** in the cpu architecture stack. It depends on:
- Clock (ticks drive `step()`)
- Hazard Detection (D03, injected via callback)
- Branch Predictor (D02, injected via `set_predict_fn`)

Used by: Core (D05).
