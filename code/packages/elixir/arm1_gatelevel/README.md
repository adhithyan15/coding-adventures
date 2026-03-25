# ARM1 Gate-Level Simulator (Elixir)

A gate-level ARM1 processor simulator where every arithmetic and logical
operation routes through actual logic gate functions (AND, OR, XOR, NOT)
chained into adders, then into a 32-bit ALU.

## How it differs from the behavioral simulator

Both simulators produce identical results for any program. The difference
is the execution path:

- **Behavioral:** opcode -> pattern match -> host arithmetic -> result
- **Gate-level:** opcode -> decoder -> barrel shifter muxes -> ALU gates -> adder gates -> result

Each ADD instruction traverses a chain of 32 full adders, each built from
XOR, AND, and OR gates. Total: ~160 gate calls per addition. The barrel
shifter uses a 5-level multiplexer tree (~640 gate calls per shift).

## Dependencies

- `coding_adventures_arm1_simulator` - types, constants, instruction encoding
- `coding_adventures_logic_gates` - AND, OR, XOR, NOT, MUX gates
- `coding_adventures_arithmetic` - ripple-carry adder built from full adders

## Usage

```elixir
alias CodingAdventures.Arm1Gatelevel, as: GL
alias CodingAdventures.Arm1Simulator, as: Sim

cpu = GL.new(4096)
cpu = GL.load_instructions(cpu, [
  Sim.encode_mov_imm(Sim.cond_al(), 0, 42),
  Sim.encode_halt()
])
{cpu, traces} = GL.run(cpu, 100)
```
