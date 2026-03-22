# Block RAM (Elixir)

Memory building blocks for digital systems, implemented in Elixir with a functional style.

## What is Block RAM?

Block RAM (BRAM) is dedicated memory embedded in FPGAs and ASICs. Unlike distributed RAM built from logic resources, BRAM is a purpose-built memory macro providing dense, fast storage.

## Module Hierarchy

```
SRAMCell          -> 1-bit storage element
  |
SRAMArray         -> M x N grid of SRAM cells
  |
SinglePortRAM     -> one read/write port
  |
DualPortRAM       -> two independent read/write ports
  |
ConfigurableBRAM  -> FPGA-style BRAM with flexible width/depth
```

## Usage

```elixir
# Single-port RAM
ram = CodingAdventures.BlockRam.SinglePortRAM.new(16, 8)
{_out, ram} = CodingAdventures.BlockRam.SinglePortRAM.access(ram, 0, [1,0,1,0,1,0,1,0], 1, 1)
{data, _ram} = CodingAdventures.BlockRam.SinglePortRAM.access(ram, 0, [0,0,0,0,0,0,0,0], 0, 1)

# Configurable BRAM
bram = CodingAdventures.BlockRam.ConfigurableBRAM.new(total_bits: 1024, width: 8)
{_out, bram} = CodingAdventures.BlockRam.ConfigurableBRAM.write(bram, 0, [1,0,1,0,1,0,1,0])
{data, _bram} = CodingAdventures.BlockRam.ConfigurableBRAM.read(bram, 0)
```

## Design

All modules use a **functional approach**: state is represented as structs, and operations return `{result, new_state}` tuples. This makes the code easy to test and reason about.

## Dependencies

- `coding_adventures_logic_gates` (sibling package)

## Running Tests

```bash
mix deps.get
mix test --cover
```
