# coding-adventures-hazard-detection (Lua)

Pipeline hazard detection: RAW data hazards, control hazards (branch misprediction), and structural hazards.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```lua
local hd = require("coding_adventures.hazard_detection")
local PipelineSlot          = hd.PipelineSlot
local DataHazardDetector    = hd.DataHazardDetector
local ControlHazardDetector = hd.ControlHazardDetector

local det = DataHazardDetector.new()

local id_slot = PipelineSlot.new({
    valid       = true,
    source_regs = {1},
    pc          = 0x8,
})
local ex_slot = PipelineSlot.new({
    valid    = true,
    dest_reg = 1,
    mem_read = true,   -- LOAD instruction
    pc       = 0x4,
})

local result = det:detect(id_slot, ex_slot, PipelineSlot.empty())
print(result.action)   -- "stall"
print(result.reason)   -- "load-use hazard: R1 is being loaded..."
```

## Dependencies

None.
