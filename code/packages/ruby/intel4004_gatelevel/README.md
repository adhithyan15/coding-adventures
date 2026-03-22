# Intel 4004 Gate-Level Simulator (Ruby)

A gate-level simulator for the Intel 4004 microprocessor where **every computation routes through real logic gates** -- no behavioral shortcuts.

## How It Works

Every operation in this simulator flows through the same gate chain that the real Intel 4004 used:

```
NOT/AND/OR/XOR -> half_adder -> full_adder -> ripple_carry_adder -> ALU
D flip-flop -> register -> register file / program counter / stack
```

When you execute `ADD R3`, the value in register R3 is read from flip-flops, the accumulator is read from flip-flops, both are fed into the ALU (which uses full adders built from gates), and the result is clocked back into the accumulator's flip-flops.

## Architecture

| Component            | Gates | Transistors |
|---------------------|-------|-------------|
| ALU (4-bit)         | 32    | 128         |
| Register file (16x4)| 480   | 1,920       |
| Accumulator (4-bit) | 24    | 96          |
| Carry flag (1-bit)  | 6     | 24          |
| Program counter (12)| 96    | 384         |
| Hardware stack (3x12)| 226  | 904         |
| Decoder             | ~50   | 200         |
| Control + wiring    | ~100  | 400         |
| **Total**           | **~1,014** | **~4,056** |

## Usage

```ruby
require "coding_adventures_intel4004_gatelevel"

cpu = CodingAdventures::Intel4004Gatelevel::Intel4004GateLevel.new

# x = 1 + 2: LDM 1, XCH R0, LDM 2, ADD R0, XCH R1, HLT
traces = cpu.run([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01])

cpu.registers[1]  # => 3 (1 + 2)
cpu.halted?       # => true
cpu.gate_count    # => ~1014
```

## Dependencies

- `coding_adventures_logic_gates` -- provides NOT, AND, OR, XOR gates and D flip-flop registers
- `coding_adventures_arithmetic` -- provides half adder, full adder, ripple-carry adder, and ALU

## How It Fits in the Stack

This package sits at the same layer as the behavioral `intel4004_simulator` but takes a fundamentally different approach. While the behavioral simulator uses Ruby arithmetic operators, this simulator routes every bit through gate functions from the logic_gates and arithmetic packages.

The cross-validation test suite runs the same programs on both simulators and verifies they produce identical results.

## Running Tests

```bash
bundle install
bundle exec rake test
```

## Linting

```bash
standardrb --fix lib/ test/
```
