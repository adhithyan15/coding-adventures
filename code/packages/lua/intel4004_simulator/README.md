# coding-adventures-intel4004-simulator (Lua)

A complete behavioral simulator for the Intel 4004 — the world's first
commercial microprocessor, released in November 1971. Implements all 46
real instructions plus a simulator-specific HLT opcode.

## Architecture

| Feature | Value |
|---------|-------|
| Data width | 4 bits (values 0–15) |
| Registers | 16 × 4-bit (R0–R15), 8 pairs (P0–P7) |
| Accumulator | 4-bit (A) |
| Carry flag | 1 bit |
| Program counter | 12 bits (4096 bytes of ROM) |
| Stack | 3-level hardware stack |
| RAM | 4 banks × 4 registers × 20 nibbles |

## Installation

```
luarocks make --local coding-adventures-intel4004-simulator-0.1.0-1.rockspec
```

## Usage

```lua
local Intel4004 = require("coding_adventures.intel4004_simulator")

local cpu = Intel4004.new()

-- x = 1 + 2
local prog = {0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01}
local traces = cpu:run(prog, 100)

print(cpu.registers[2])   -- 3 (result in R1)

-- Inspect trace
for _, t in ipairs(traces) do
    print(string.format("0x%03X: %s  A: %d->%d",
        t.address, t.mnemonic, t.accumulator_before, t.accumulator_after))
end
```
