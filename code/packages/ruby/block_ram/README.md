# coding_adventures_block_ram

Block RAM implementation for the coding-adventures project. Provides SRAM cells, arrays, and synchronous RAM modules used in FPGAs and CPU caches.

## Components

- **SRAMCell** -- Single-bit storage element modeled at the gate level (6T SRAM cell)
- **SRAMArray** -- 2D grid of SRAM cells with row/column addressing
- **SinglePortRAM** -- Synchronous RAM with one address port and configurable read modes
- **DualPortRAM** -- True dual-port RAM with independent A and B ports
- **ConfigurableBRAM** -- FPGA-style Block RAM with reconfigurable aspect ratio

## Usage

```ruby
require "coding_adventures_block_ram"

# Single-port RAM: 256 words x 8 bits
ram = CodingAdventures::BlockRam::SinglePortRAM.new(depth: 256, width: 8)

# Write [1,0,1,0,1,0,1,0] to address 0
ram.tick(0, address: 0, data_in: [1,0,1,0,1,0,1,0], write_enable: 1)
ram.tick(1, address: 0, data_in: [1,0,1,0,1,0,1,0], write_enable: 1)

# Read from address 0
ram.tick(0, address: 0, data_in: [0]*8, write_enable: 0)
out = ram.tick(1, address: 0, data_in: [0]*8, write_enable: 0)
# out => [1, 0, 1, 0, 1, 0, 1, 0]

# Configurable BRAM (FPGA-style)
bram = CodingAdventures::BlockRam::ConfigurableBRAM.new(total_bits: 1024, width: 8)
bram.depth  # => 128
bram.reconfigure(width: 16)
bram.depth  # => 64
```

## Dependencies

- `coding_adventures_logic_gates` -- fundamental logic gates

## Layer

Layer 11 of the computing stack (built on logic gates from Layer 10).
