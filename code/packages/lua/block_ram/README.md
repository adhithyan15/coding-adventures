# block-ram

SRAM cells, arrays, and RAM modules — the memory building blocks for CPUs,
caches, and FPGA Block RAM. Ported from the Go implementation.

## Layer 8

This package is part of Layer 8 of the coding-adventures computing stack.
It builds a complete memory hierarchy from single-bit SRAM cells up to
configurable dual-port Block RAM.

## Components

| Component | Description |
|-----------|-------------|
| `SRAMCell` | Single-bit storage (6-transistor model) |
| `SRAMArray` | 2D grid of cells with row/column addressing |
| `SinglePortRAM` | Synchronous RAM with one port and three read modes |
| `DualPortRAM` | True dual-port RAM with collision detection |
| `ConfigurableBRAM` | FPGA-style Block RAM with reconfigurable aspect ratio |

## Dependencies

- `logic-gates` (conceptual dependency; validation helpers are self-contained)

## Usage

```lua
local block_ram = require("coding_adventures.block_ram")

-- Create a 256-word x 8-bit single-port RAM (read-first mode)
local ram = block_ram.SinglePortRAM.new(256, 8, block_ram.READ_FIRST)

-- Write {1,0,1,0,0,1,0,1} to address 0 on a rising clock edge
ram:tick(0, 0, {1,0,1,0,0,1,0,1}, 1)  -- clock low (setup)
ram:tick(1, 0, {1,0,1,0,0,1,0,1}, 1)  -- rising edge triggers write

-- Read back from address 0
ram:tick(0, 0, {0,0,0,0,0,0,0,0}, 0)  -- clock low
local data = ram:tick(1, 0, {0,0,0,0,0,0,0,0}, 0)  -- rising edge reads
-- data = {1,0,1,0,0,1,0,1}

-- Dual-port RAM: two independent ports
local dp = block_ram.DualPortRAM.new(8, 4,
    block_ram.READ_FIRST, block_ram.WRITE_FIRST)
local out_a, out_b, err = dp:tick(1,
    0, {1,0,1,0}, 1,   -- port A writes to addr 0
    1, {0,1,0,1}, 1)   -- port B writes to addr 1
-- err is nil (no collision, different addresses)

-- Configurable Block RAM (FPGA-style)
local bram = block_ram.ConfigurableBRAM.new(1024, 8)  -- 1024 bits, 8-bit words
-- depth=128, width=8
bram:reconfigure(16)  -- now depth=64, width=16 (data cleared)
```

## Read Modes

During a write, the data output behavior depends on the read mode:

| Mode | `data_out` during write | Use case |
|------|------------------------|----------|
| `READ_FIRST` | Old value at address | Know what was there before overwriting |
| `WRITE_FIRST` | New value being written | Pipeline forwarding |
| `NO_CHANGE` | Previous read output | Power savings (FPGA Block RAM) |

## Development

```bash
# Run tests (from package root)
cd tests && busted . --verbose --pattern=test_
```
