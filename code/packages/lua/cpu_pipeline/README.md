# coding-adventures-cpu-pipeline (Lua)

Configurable N-stage CPU instruction pipeline.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```lua
local cpu_pipeline   = require("coding_adventures.cpu_pipeline")
local Pipeline       = cpu_pipeline.Pipeline
local PipelineConfig = cpu_pipeline.PipelineConfig
local HazardResponse = cpu_pipeline.HazardResponse

local memory = {0xFF, 0, 0, 0}  -- HALT at address 0

local result = Pipeline.new(
    PipelineConfig.classic_5_stage(),
    function(pc) return memory[pc + 1] or 0 end,  -- fetch
    function(raw, tok)                             -- decode
        if raw == 0xFF then tok.opcode = "HALT"; tok.is_halt = true
        else tok.opcode = "NOP" end
        return tok
    end,
    function(tok) return tok end,   -- execute
    function(tok) return tok end,   -- memory
    function(tok) end               -- writeback
)
local p = result.pipeline
p:run(100)
print(p:is_halted())  -- true
print(string.format("IPC: %.3f", p:get_stats():ipc()))
```

## Dependencies

None (no external Lua dependencies).
