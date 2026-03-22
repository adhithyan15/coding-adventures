# block-ram

**Block RAM** — SRAM cells, arrays, and configurable RAM modules for FPGA simulation.

## What is this?

This crate models memory at the gate level, building up from individual SRAM cells to complete RAM modules suitable for use in FPGA simulation. It follows the real hardware hierarchy:

```
SRAMCell        -- 1-bit storage (cross-coupled inverters + access transistors)
    |
SRAMArray       -- 2D grid of cells with row/column addressing
    |
SinglePortRAM   -- synchronous RAM with one read/write port
DualPortRAM     -- synchronous RAM with two independent ports
    |
ConfigurableBRAM -- FPGA Block RAM with reconfigurable aspect ratio
```

## How it fits in the stack

`block-ram` sits between `logic-gates` and `fpga`:

- **logic-gates** provides the primitive gates and sequential elements
- **block-ram** (this crate) provides memory storage
- **fpga** uses block-ram for on-chip memory tiles

## Key types

| Type | Description |
|------|-------------|
| `SRAMCell` | Single-bit storage element (6T SRAM cell model) |
| `SRAMArray` | 2D grid of cells with row/column addressing |
| `SinglePortRAM` | Synchronous RAM with one read/write port and configurable read mode |
| `DualPortRAM` | True dual-port RAM with write collision detection |
| `ConfigurableBRAM` | FPGA Block RAM with reconfigurable width/depth aspect ratio |
| `ReadMode` | Controls data_out during writes: ReadFirst, WriteFirst, NoChange |

## Usage

```rust
use block_ram::ram::{SinglePortRAM, ReadMode};

let mut ram = SinglePortRAM::new(256, 8, ReadMode::ReadFirst);

// Write 0xFF to address 0 (rising edge: clock 0 -> 1)
ram.tick(0, 0, &[1,1,1,1,1,1,1,1], 1);
ram.tick(1, 0, &[1,1,1,1,1,1,1,1], 1);

// Read from address 0
ram.tick(0, 0, &[0;8], 0);
let data = ram.tick(1, 0, &[0;8], 0);
assert_eq!(data, vec![1,1,1,1,1,1,1,1]);
```

## Dependencies

- `logic-gates` (path dependency)
