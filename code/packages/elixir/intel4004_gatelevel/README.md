# Intel 4004 Gate-Level Simulator (Elixir)

A gate-level simulation of the Intel 4004 microprocessor. Every operation routes through real logic gate functions — AND, OR, NOT, XOR gates for combinational logic, and D flip-flops for state storage.

## Where This Fits

```
Layer 7d2: Intel 4004 Gate-Level  ← this package
    ↓ uses
Layer 4: Arithmetic (adders, ALU)
Layer 3: Logic Gates (AND, OR, NOT, XOR)
Layer 2: Transistors
Layer 1: NAND gates
```

Unlike the behavioral simulator (which directly implements instruction semantics), this version builds the CPU from primitives:
- **ALU** uses `Arithmetic.alu_execute/3` which chains half adders → full adders → ripple carry adder
- **Registers** use `Sequential.register/3` with D flip-flop state maps
- **Decoder** uses `Gates.and_gate/2`, `Gates.or_gate/2`, `Gates.not_gate/1` networks
- **Program counter** increments via a chain of `Arithmetic.half_adder/2` calls

## Usage

```elixir
alias CodingAdventures.Intel4004GateLevel, as: GL

# x = 1 + 2
{cpu, traces} = GL.run(<<0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01>>)
GL.accumulator(cpu)           # => 0 (after XCH)
Enum.at(GL.registers(cpu), 1) # => 3

# Gate count estimate
GL.gate_count()  # => 8894
```

## Architecture

All state is stored as flip-flop state maps (`%{master_q, master_q_bar, slave_q, slave_q_bar}`). Writing requires a two-phase clock cycle: clock=0 captures data into the master latch, clock=1 transfers it to the slave latch. This mirrors how real edge-triggered flip-flops work.

The immutable struct holds:
- 16 x 4-bit register flip-flop states
- 4-bit accumulator flip-flop state
- 1-bit carry flag flip-flop state
- 12-bit program counter flip-flop state
- 3 x 12-bit hardware call stack flip-flop states
- RAM stored as a map of flip-flop states

## Running Tests

```bash
mix test
```
