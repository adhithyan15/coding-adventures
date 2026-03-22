# FPGA (Elixir)

Field-Programmable Gate Array simulation in Elixir.

## What is an FPGA?

An FPGA is a chip that can be programmed to implement any digital circuit after manufacturing. It contains a grid of reconfigurable logic blocks connected by a programmable routing network.

## Module Hierarchy

```
LUT           -> Lookup Table (truth table stored in SRAM)
  |
Slice         -> 2 LUTs + 2 Flip-Flops + Carry Chain
  |
CLB           -> 2 Slices (Configurable Logic Block)
  |
SwitchMatrix  -> Programmable routing crossbar
  |
IOBlock       -> Input/Output interface to external pins
  |
Bitstream     -> Configuration data (from Elixir maps)
  |
Fabric        -> Complete FPGA with all components
```

## Usage

```elixir
# Create a 2x2 FPGA fabric with 2-input LUTs
fabric = CodingAdventures.FPGA.Fabric.new(2, 2, lut_inputs: 2)

# Load a bitstream configuration
bs = CodingAdventures.FPGA.Bitstream.from_map(%{
  "clbs" => %{
    "0_0" => %{
      "slice_0" => %{"lut_a" => [0, 0, 0, 1]}  # AND gate
    }
  },
  "routing" => %{},
  "io" => %{}
})
fabric = CodingAdventures.FPGA.Fabric.load_bitstream(fabric, bs)

# Set inputs and evaluate
fabric = CodingAdventures.FPGA.Fabric.set_input(fabric, "top_0", 1)
fabric = CodingAdventures.FPGA.Fabric.evaluate(fabric, 0)
```

## Dependencies

- `coding_adventures_logic_gates` (sibling package)
- `coding_adventures_block_ram` (sibling package)

## Running Tests

```bash
mix deps.get
mix test --cover
```
