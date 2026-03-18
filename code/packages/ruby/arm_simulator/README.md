# ARM Simulator

Simulates a subset of the ARMv7 instruction set: MOV immediate, ADD register,
SUB register, and HLT. Part of the **coding-adventures** computing stack.

## How it fits in the stack

```
Layer 8: CPU Simulator (generic CPU)
Layer 7: ARM Simulator  <-- this gem
```

## Usage

```ruby
require "coding_adventures_arm_simulator"

sim = CodingAdventures::ArmSimulator::ARMSimulator.new
program = CodingAdventures::ArmSimulator.assemble([
  CodingAdventures::ArmSimulator.encode_mov_imm(0, 1),
  CodingAdventures::ArmSimulator.encode_mov_imm(1, 2),
  CodingAdventures::ArmSimulator.encode_add(2, 0, 1),
  CodingAdventures::ArmSimulator.encode_hlt
])
traces = sim.run(program)
puts sim.cpu.registers.read(2) # => 3
```
