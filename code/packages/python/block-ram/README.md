# block-ram

Hardware-level read/write memory arrays — SRAM cells, arrays, single/dual-port RAM, and configurable Block RAM.

## What is this?

This package models the memory hardware found inside FPGAs and ASICs, built from the ground up:

1. **SRAM Cell** — a single bit of storage modeled after the 6-transistor SRAM cell (cross-coupled inverters + access transistors)
2. **SRAM Array** — a 2D grid of SRAM cells with row/column addressing
3. **Single-Port RAM** — synchronous memory with one address port, supporting three read modes (READ_FIRST, WRITE_FIRST, NO_CHANGE)
4. **Dual-Port RAM** — two independent ports for simultaneous read/write operations, with write collision detection
5. **Configurable BRAM** — FPGA-style Block RAM with reconfigurable aspect ratio (trade depth for width while keeping total storage fixed)

## How it fits in the stack

```
logic-gates (NOT, AND, OR — primitive gates)
    │
    ├── arithmetic (adders, ALU)
    ├── clock (clock generator, divider)
    └── block-ram ← YOU ARE HERE (SRAM cells → RAM modules → Block RAM)
            │
            └── fpga (LUT, CLB, routing fabric — coming soon)
```

Block RAM is the memory foundation that FPGA Block RAM tiles are built on. The existing `rom-bios` package provides read-only memory; this package adds writable storage.

## Usage

```python
from block_ram import SRAMCell, SRAMArray, SinglePortRAM, DualPortRAM, ConfigurableBRAM, ReadMode

# === SRAM Cell (single bit) ===
cell = SRAMCell()
cell.write(word_line=1, bit_line=1)  # Store a 1
cell.read(word_line=1)               # Returns 1
cell.read(word_line=0)               # Returns None (not selected)

# === SRAM Array (2D grid) ===
arr = SRAMArray(rows=4, cols=8)
arr.write(0, [1, 0, 1, 0, 0, 1, 0, 1])
arr.read(0)  # [1, 0, 1, 0, 0, 1, 0, 1]

# === Single-Port RAM (synchronous) ===
ram = SinglePortRAM(depth=256, width=8, read_mode=ReadMode.READ_FIRST)
# Write 0xFF to address 0 (rising edge = clock 0 then 1)
ram.tick(0, address=0, data_in=[1]*8, write_enable=1)
ram.tick(1, address=0, data_in=[1]*8, write_enable=1)
# Read it back
ram.tick(0, address=0, data_in=[0]*8, write_enable=0)
out = ram.tick(1, address=0, data_in=[0]*8, write_enable=0)
# out == [1, 1, 1, 1, 1, 1, 1, 1]

# === Dual-Port RAM (two independent ports) ===
dpram = DualPortRAM(depth=256, width=8)
# Port A writes address 0 while Port B reads address 1 — simultaneously
dpram.tick(0, address_a=0, data_in_a=[1]*8, write_enable_a=1,
              address_b=1, data_in_b=[0]*8, write_enable_b=0)
out_a, out_b = dpram.tick(1, address_a=0, data_in_a=[1]*8, write_enable_a=1,
                              address_b=1, data_in_b=[0]*8, write_enable_b=0)

# === Configurable BRAM (reconfigurable aspect ratio) ===
bram = ConfigurableBRAM(total_bits=1024, width=8)
bram.depth   # 128 words × 8 bits
bram.reconfigure(width=16)
bram.depth   # 64 words × 16 bits (same 1024 total bits)
```

## Read Modes

During a write operation, what should the data output show?

| Mode | Output during write | Use case |
|------|-------------------|----------|
| `READ_FIRST` | Old value at address | Need to know previous value |
| `WRITE_FIRST` | New value being written | Pipeline forwarding |
| `NO_CHANGE` | Previous read output | Power savings in FPGAs |

## Installation

```bash
pip install coding-adventures-block-ram
```

## Development

```bash
uv venv && uv pip install -e ".[dev]"
pytest
ruff check src/ tests/
mypy src/
```
