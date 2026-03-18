# RISC-V Simulator

Simulates a subset of RISC-V RV32I: addi, add, sub, ecall. Part of the
**coding-adventures** computing stack.

## Usage

```ruby
require "coding_adventures_riscv_simulator"

sim = CodingAdventures::RiscvSimulator::RiscVSimulator.new
program = CodingAdventures::RiscvSimulator.assemble([
  CodingAdventures::RiscvSimulator.encode_addi(1, 0, 1),
  CodingAdventures::RiscvSimulator.encode_addi(2, 0, 2),
  CodingAdventures::RiscvSimulator.encode_add(3, 1, 2),
  CodingAdventures::RiscvSimulator.encode_ecall
])
traces = sim.run(program)
puts sim.cpu.registers.read(3) # => 3
```
