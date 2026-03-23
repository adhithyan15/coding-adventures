# coding_adventures_fpga

FPGA fabric model for the coding-adventures project. Implements a simplified but structurally accurate FPGA with programmable logic, routing, and I/O.

## Components

- **LUT** -- K-input Look-Up Table, the atom of programmable logic. Stores a truth table in SRAM and uses a MUX tree to select the output.
- **Slice** -- 2 LUTs + 2 flip-flops + carry chain. The building block of a CLB.
- **CLB** -- Configurable Logic Block with 2 slices. The core compute tile.
- **SwitchMatrix** -- Programmable routing crossbar connecting CLBs.
- **IOBlock** -- Bidirectional I/O pad (input, output, or tri-state mode).
- **Bitstream** -- JSON-based configuration that programs the entire fabric.
- **FPGAFabric** -- Top-level model tying everything together.

## Usage

```ruby
require "coding_adventures_fpga"

# Create a LUT configured as a 2-input AND gate
and_table = [0] * 16
and_table[3] = 1  # I0=1, I1=1 -> output=1
lut = CodingAdventures::FPGA::LUT.new(k: 4, truth_table: and_table)
lut.evaluate([1, 1, 0, 0])  # => 1
lut.evaluate([1, 0, 0, 0])  # => 0

# Configure a full FPGA from JSON
bs = CodingAdventures::FPGA::Bitstream.from_hash({
  "clbs" => { "clb_0" => { "slice0" => { "lut_a" => and_table } } },
  "io" => { "in_a" => { "mode" => "input" }, "out" => { "mode" => "output" } }
})
fpga = CodingAdventures::FPGA::FPGAFabric.new(bs)
```

## Dependencies

- `coding_adventures_logic_gates` -- fundamental logic gates and combinational circuits
- `coding_adventures_block_ram` -- SRAM cells for LUT storage

## Layer

Layer 12 of the computing stack (built on logic gates and block RAM).
