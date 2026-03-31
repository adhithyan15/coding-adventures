# coding-adventures-fpga (Lua)

FPGA fabric simulation — LUTs, Slices, CLBs, Switch Matrices, I/O Blocks, and Bitstreams in pure Lua 5.4.

## Architecture

```
Bitstream (configuration data)
    │
    ▼
Fabric (rows × cols CLB grid)
    │
    ├── CLB (row, col)
    │     ├── Slice 0 → LUT_A, LUT_B, FF_A, FF_B, carry chain
    │     └── Slice 1 → LUT_A, LUT_B, FF_A, FF_B, carry chain
    │
    ├── SwitchMatrix (per CLB) — programmable routing
    │
    └── IOBlock (perimeter) — input/output/bidirectional
```

## Installation

```bash
luarocks make --local coding-adventures-fpga-0.1.0-1.rockspec
```

## Usage

```lua
local FPGA = require("coding_adventures.fpga")

-- Program a 2-input AND gate in a LUT
local lut = FPGA.LUT.new(2)
lut:configure({0, 0, 0, 1})  -- AND truth table
print(lut:evaluate({1, 1}))  -- 1

-- Full fabric with bitstream
local fab = FPGA.Fabric.new(2, 2, {lut_inputs = 4})
local bs = FPGA.Bitstream.from_map({
    clbs = {
        ["0,0"] = {
            slice_0 = { lut_a = {0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,1} }
        }
    }
})
fab:load_bitstream(bs)
fab:set_input("top_0", 1)
fab:evaluate(0)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
