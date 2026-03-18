# CLR Simulator

Simulates CLR IL bytecode with real opcode values: ldc.i4 variants, ldloc/stloc,
add/sub/mul/div, br.s, brfalse.s, brtrue.s, ceq/cgt/clt, ret, nop. Part of the
**coding-adventures** computing stack.

## Usage

```ruby
require "coding_adventures_clr_simulator"

sim = CodingAdventures::ClrSimulator::CLRSimulator.new
sim.load(CodingAdventures::ClrSimulator.assemble_clr(
  CodingAdventures::ClrSimulator.encode_ldc_i4(1),
  CodingAdventures::ClrSimulator.encode_ldc_i4(2),
  [CodingAdventures::ClrSimulator::ADD],
  CodingAdventures::ClrSimulator.encode_stloc(0),
  [CodingAdventures::ClrSimulator::RET]
))
traces = sim.run
puts sim.locals[0] # => 3
```
