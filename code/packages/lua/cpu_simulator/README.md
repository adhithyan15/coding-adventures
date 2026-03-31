# coding-adventures-cpu-simulator (Lua)

CPU simulator building blocks: byte-addressable Memory, SparseMemory, and RegisterFile.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Synopsis

```lua
local cpu_sim      = require("coding_adventures.cpu_simulator")
local Memory       = cpu_sim.Memory
local SparseMemory = cpu_sim.SparseMemory
local RegisterFile = cpu_sim.RegisterFile

-- Dense memory (64 KB)
local m = Memory.new(65536)
m:write_word(0, 0xDEADBEEF)
print(string.format("0x%08X", m:read_word(0)))  -- 0xDEADBEEF

-- Sparse memory (huge address space, minimal allocation)
local sm = SparseMemory.new()
sm:write_byte(0xFFFF0000, 0x42)
print(sm:read_byte(0xFFFF0000))  -- 66

-- Register file (16 × 32-bit registers)
local rf = RegisterFile.new(16, 32)
rf:write(1, 0xCAFE)
print(rf:read(1))  -- 51966
```

## Dependencies

None.
