# block-ram

A Go package implementing SRAM cells, arrays, and RAM modules — the memory building blocks for CPUs, caches, and FPGA Block RAM.

## Where this fits in the stack

```
Layer 0: Logic Gates
Layer 1: Block RAM         <-- you are here
Layer 2: FPGA (uses Block RAM for storage)
Layer 3: CPU / GPU (uses RAM for caches and register files)
```

Block RAM sits between logic gates and the FPGA fabric. It provides the memory elements that FPGAs use to store truth tables (in LUTs), configuration bits, and user data.

## What's included

### SRAM (`sram.go`)

| Component | Description |
|-----------|-------------|
| `SRAMCell` | Single-bit storage element (6T SRAM cell model) |
| `SRAMArray` | 2D grid of cells with row/column addressing |

### RAM Modules (`ram.go`)

| Component | Description |
|-----------|-------------|
| `SinglePortRAM` | One address port, synchronous read/write with 3 read modes |
| `DualPortRAM` | Two independent ports with collision detection |
| `ReadMode` | `ReadFirst`, `WriteFirst`, `NoChange` |
| `WriteCollisionError` | Returned on dual-port write collision |

### Configurable Block RAM (`bram.go`)

| Component | Description |
|-----------|-------------|
| `ConfigurableBRAM` | FPGA-style BRAM with reconfigurable width/depth ratio |

## Usage

```go
import blockram "github.com/adhithyan15/coding-adventures/code/packages/go/block-ram"

// SRAM Cell
cell := blockram.NewSRAMCell()
cell.Write(1, 1)  // word_line=1, bit_line=1
val := cell.Read(1) // returns &1

// Single-Port RAM (256 words x 8 bits, read-first mode)
ram := blockram.NewSinglePortRAM(256, 8, blockram.ReadFirst)
data := []int{1, 0, 1, 0, 0, 1, 0, 1}
ram.Tick(0, 0, data, 1)  // clock LOW
ram.Tick(1, 0, data, 1)  // clock HIGH (rising edge: write)

// Configurable Block RAM (1024 bits, 8-bit width)
bram := blockram.NewConfigurableBRAM(1024, 8)
// depth = 128 words
bram.Reconfigure(16)
// now: depth = 64 words, width = 16 bits
```

## Read Modes

During a write operation, the data output depends on the read mode:

| Mode | Output during write | Use case |
|------|-------------------|----------|
| `ReadFirst` | Old value at address | Need previous value before overwrite |
| `WriteFirst` | New value being written | Pipeline forwarding |
| `NoChange` | Previous read value (unchanged) | Power savings in FPGA |

## Testing

```bash
go test ./... -v -cover
```

## Literate programming

All source files use Knuth-style literate programming with extensive comments explaining:
- How SRAM cells work at the transistor level
- The 6T cell architecture
- Read mode behaviors with timing diagrams
- FPGA Block RAM configuration tables
