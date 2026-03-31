# coding-adventures-core (Lua)

Complete CPU core integrating pipeline, register file, and memory controller with ISA decoder injection.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```lua
local core_mod = require("coding_adventures.core")
local Core       = core_mod.Core
local CoreConfig = core_mod.CoreConfig

-- Implement a minimal ISA decoder
local MyDecoder = {}
MyDecoder.__index = MyDecoder
function MyDecoder.new() return setmetatable({}, MyDecoder) end
function MyDecoder:decode(raw, token)
    token.opcode  = raw == 0xFF and "HALT" or "NOP"
    token.is_halt = raw == 0xFF
    return token
end
function MyDecoder:execute(token, reg_file) return token end
function MyDecoder:instruction_size() return 4 end

local result = Core.new(CoreConfig.simple(), MyDecoder.new())
local core   = result.core
core:load_program({0xFF, 0, 0, 0}, 0)  -- HALT program
core:run(100)
print(core:is_halted())           -- true
print(core:get_stats():ipc())    -- e.g. 0.200
```

## Dependencies

- `coding-adventures-cpu-pipeline` — pipeline engine
- `coding-adventures-cpu-simulator` — Memory and RegisterFile
