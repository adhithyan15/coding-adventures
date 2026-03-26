# Intel 4004 Simulator

Simulates the Intel 4004, the world's first commercial microprocessor (1971).
All 46 real instructions are implemented, plus HLT (simulator-only halt).
4-bit values, 16 registers, accumulator architecture, 3-level hardware call
stack, full RAM/ROM I/O model. Part of the **coding-adventures** computing stack.

## Architecture

| Component          | Size / Range               |
|--------------------|----------------------------|
| Data width         | 4 bits (values 0-15)       |
| Instruction width  | 8 bits (some 2-byte)       |
| Registers          | 16 x 4-bit (R0-R15)       |
| Register pairs     | 8 x 8-bit (P0-P7)         |
| Accumulator        | 4-bit                      |
| Carry flag         | 1 bit                      |
| Program counter    | 12 bits (0-4095)           |
| Call stack          | 3 levels x 12-bit          |
| ROM                | 4096 x 8-bit               |
| RAM                | 4 banks x 4 regs x 20 nib |

## Usage

```ruby
require "coding_adventures_intel4004_simulator"

sim = CodingAdventures::Intel4004Simulator::Intel4004Sim.new

# x = 1 + 2: LDM 1, XCH R0, LDM 2, ADD R0, XCH R1, HLT
program = [0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01].pack("C*")
traces = sim.run(program)
puts sim.registers[1] # => 3

# Each trace records before/after state:
traces.each do |t|
  puts "#{format('%03X', t.address)}: #{t.mnemonic.ljust(12)} A=#{t.accumulator_after}"
end
```

## Instruction Set (46 instructions)

### Control
- `NOP` (0x00) -- No operation
- `HLT` (0x01) -- Halt (simulator-only)

### Jumps
- `JCN c,addr` (0x1C AA) -- Conditional jump
- `JUN addr` (0x4H LL) -- Unconditional jump (12-bit)
- `JMS addr` (0x5H LL) -- Jump to subroutine
- `ISZ Rn,addr` (0x7R AA) -- Increment and skip if zero
- `JIN Pp` (0x3P odd) -- Jump indirect via register pair
- `BBL n` (0xCN) -- Branch back (return) and load

### Register
- `LDM n` (0xDN) -- Load immediate into A
- `LD Rn` (0xAN) -- Load register into A
- `XCH Rn` (0xBN) -- Exchange A with register
- `INC Rn` (0x6N) -- Increment register

### Register Pair
- `FIM Pp,data` (0x2P DD) -- Fetch immediate to pair
- `SRC Pp` (0x2P+1) -- Send register control
- `FIN Pp` (0x3P even) -- Fetch indirect from ROM

### Arithmetic
- `ADD Rn` (0x8N) -- Add register to A with carry
- `SUB Rn` (0x9N) -- Subtract register from A (complement-add)
- `ADM` (0xEB) -- Add RAM to A with carry
- `SBM` (0xE8) -- Subtract RAM from A

### I/O
- `WRM` (0xE0) -- Write A to RAM main character
- `WMP` (0xE1) -- Write A to RAM output port
- `WRR` (0xE2) -- Write A to ROM I/O port
- `WPM` (0xE3) -- Write program RAM (NOP in simulator)
- `WR0`-`WR3` (0xE4-E7) -- Write A to RAM status
- `RDM` (0xE9) -- Read RAM main character into A
- `RDR` (0xEA) -- Read ROM I/O port into A
- `RD0`-`RD3` (0xEC-EF) -- Read RAM status into A

### Accumulator
- `CLB` (0xF0) -- Clear A and carry
- `CLC` (0xF1) -- Clear carry
- `IAC` (0xF2) -- Increment A
- `CMC` (0xF3) -- Complement carry
- `CMA` (0xF4) -- Complement A (4-bit NOT)
- `RAL` (0xF5) -- Rotate left through carry
- `RAR` (0xF6) -- Rotate right through carry
- `TCC` (0xF7) -- Transfer carry to A, clear carry
- `DAC` (0xF8) -- Decrement A
- `TCS` (0xF9) -- Transfer carry subtract (A=10 if carry, else 9)
- `STC` (0xFA) -- Set carry
- `DAA` (0xFB) -- Decimal adjust (BCD correction)
- `KBP` (0xFC) -- Keyboard process (1-hot to binary)
- `DCL` (0xFD) -- Designate command line (select RAM bank)

## Key Behaviors

- **SUB uses complement-add**: `A + NOT(Rn) + borrow_in`, where carry=true means NO borrow
- **ADD includes carry**: `A + Rn + carry`, enabling multi-precision arithmetic
- **ISZ**: increment register, jump if NOT zero (loop counter pattern)
- **JCN condition nibble**: bit 3=invert, bit 2=test_zero, bit 1=test_carry, bit 0=test_pin
- **3-level stack** wraps silently on overflow (no exception)
- **KBP truth table**: {0->0, 1->1, 2->2, 4->3, 8->4, else->15}
- **DAA**: if A>9 or carry, add 6; if overflow, set carry

## Testing

```bash
bundle exec rake test
```
