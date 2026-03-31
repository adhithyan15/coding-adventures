# coding-adventures-fpga (Lua)

FPGA (Field-Programmable Gate Array) simulation — programmable hardware from the ground up.

## What is an FPGA?

An FPGA is a chip full of logic gates, memory, and wires — but unlike a CPU or GPU where the circuits are permanently etched in silicon, an FPGA's circuits are **programmable**. You upload a configuration file (called a **bitstream**) and the chip reconfigures itself to implement whatever digital circuit you described.

**The key insight:** a Lookup Table (LUT) storing a truth table is functionally identical to a logic gate — but which gate it implements is determined by the truth table contents, not physical structure. The same silicon becomes AND, OR, XOR, or any custom function by loading a different truth table. **A truth table is a program.**

## Package Structure

```
src/coding_adventures/fpga/
  init.lua         -- top-level module (re-exports everything)
  lut.lua          -- LUT: truth-table lookup element
  slice.lua        -- Slice: 2 LUTs + 2 FFs + carry chain
  clb.lua          -- CLB: 2 slices (Configurable Logic Block)
  switch_matrix.lua -- SwitchMatrix: programmable routing crossbar
  io_block.lua     -- IOBlock: external pin interface
  fabric.lua       -- Fabric: complete FPGA (CLB grid + routing + I/O)
  bitstream.lua    -- Bitstream: configuration parser
tests/
  test_fpga.lua    -- comprehensive tests
```

## Installation

```bash
luarocks make --local coding-adventures-fpga-0.1.0-1.rockspec
```

## Usage

```lua
local fpga = require("coding_adventures.fpga")

-- Create a 2x2 fabric with 4-input LUTs
local fabric = fpga.Fabric.new(2, 2)

-- Or use individual components:
local lut = fpga.LUT.new(4)
lut:configure({0,0,0,0, 0,0,0,1, 0,0,0,0, 0,0,0,0})  -- 4-input AND
print(lut:evaluate({0,1,1,1}))  -- 0
print(lut:evaluate({1,1,1,1}))  -- 1
```

## Component API

### LUT

```lua
local lut = LUT.new(num_inputs)         -- create with num_inputs
lut:configure(truth_table)              -- load truth table (2^n bits)
local bit = lut:evaluate(inputs)        -- evaluate (inputs = array of 0/1)
```

### Slice

```lua
local s = Slice.new(opts)
-- opts: lut_inputs, use_ff_a, use_ff_b, carry_enable
s:configure({ lut_a = tt, lut_b = tt })
local out_a, out_b, carry_out = s:evaluate(inputs_a, inputs_b, clock, carry_in)
```

### CLB

```lua
local clb = CLB.new(row, col, opts)
clb:configure({ slice_0 = { lut_a = tt }, slice_1 = { lut_b = tt } })
local outputs, carry_out = clb:evaluate(inputs, clock, carry_in)
-- outputs = {s0_a, s0_b, s1_a, s1_b}
```

### SwitchMatrix

```lua
local sm = SwitchMatrix.new(num_inputs, num_outputs)
sm:configure({ out_0 = "in_2", out_1 = "in_0" })  -- routing map
local signals = sm:route({ in_0 = 1, in_1 = 0, in_2 = 1, in_3 = 0 })
-- signals = { out_0 = 1, out_1 = 1, out_2 = nil, out_3 = nil }
```

### IOBlock

```lua
local io = IOBlock.new("pin_0", "input")    -- "input", "output", or "bidirectional"
io:set_pin(1)                               -- for input/bidirectional blocks
local v = io:read_fabric()                  -- value seen by internal fabric
local v = io:read_pin()                     -- value on external pin
```

### Fabric

```lua
local f = Fabric.new(rows, cols, opts)
-- opts: lut_inputs, switch_size
f:load_bitstream(bs)                 -- apply configuration
f:set_input("top_0", 1)             -- drive an input pin
local v = f:read_output("bottom_0") -- read an output pin
f:evaluate(clock)                    -- tick the clock
local s = f:summary()               -- resource counts table
```

### Bitstream

```lua
local bs = Bitstream.from_map({
  clbs = {
    ["0_0"] = {
      slice_0 = { lut_a = {0,0,0,1}, lut_b = {0,1,1,0} },
      slice_1 = { lut_a = {0,1,1,1} },
    }
  },
  routing = {
    ["0_0"] = { out_0 = "in_2" }
  },
  io = {
    top_0 = { direction = "input" }
  }
})
local cfg = bs:clb_config("0_0")
local rtr = bs:routing_config("0_0")
local iocfg = bs:io_config("top_0")
```

## Where It Fits

```
logic-gates → combinational → block-ram → [YOU ARE HERE]
                                           ↑
                               clock ──────┘
                               arithmetic ─┘ (carry chains)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
