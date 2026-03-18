# Intel 4004 Simulator

Simulates the Intel 4004, the world's first commercial microprocessor (1971).
4-bit values, 16 registers, accumulator architecture. Part of the
**coding-adventures** computing stack.

## Usage

```ruby
require "coding_adventures_intel4004_simulator"

sim = CodingAdventures::Intel4004Simulator::Intel4004Sim.new
# x = 1 + 2: LDM 1, XCH R0, LDM 2, ADD R0, XCH R1, HLT
program = [0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01].pack("C*")
traces = sim.run(program)
puts sim.registers[1] # => 3
```
