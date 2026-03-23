# Intel 4004 Simulator (Elixir)

A complete simulator for the Intel 4004 — the world's first commercial microprocessor (1971). Implements all 46 real instructions plus HLT using idiomatic Elixir with immutable state.

## Where This Fits

```
Layer 7d: Intel 4004 Simulator
    ↓ uses
Layer 7: Virtual Machine (GenericVM)    [not used — standalone]
    ↓ sits on
Layer 6: Clock
Layer 5: Sequential Logic (flip-flops)
Layer 4: Arithmetic (adders, ALU)
Layer 3: Logic Gates (AND, OR, NOT)
Layer 2: Transistors
Layer 1: NAND gates
```

The behavioral simulator is standalone — it directly implements instruction semantics without routing through lower layers. The gate-level simulator (separate package) uses the real gate/ALU/register primitives.

## Usage

```elixir
alias CodingAdventures.Intel4004Simulator, as: Sim

# x = 1 + 2
{cpu, traces} = Sim.run(<<0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01>>)
Enum.at(cpu.registers, 1)  # => 3

# Inspect execution trace
for t <- traces do
  IO.puts("#{String.pad_leading(Integer.to_string(t.address, 16), 3, "0")}: #{t.mnemonic}")
end
```

## Complete Instruction Set (46 instructions)

| Opcode | Mnemonic | Description |
|--------|----------|-------------|
| 0x00 | NOP | No operation |
| 0x01 | HLT | Halt (simulator-only) |
| 0x1_ | JCN c,a | Conditional jump |
| 0x2_ even | FIM Pp,d | Fetch immediate to pair |
| 0x2_ odd | SRC Pp | Send register control |
| 0x3_ even | FIN Pp | Fetch indirect from ROM |
| 0x3_ odd | JIN Pp | Jump indirect |
| 0x4_ | JUN a | Unconditional jump |
| 0x5_ | JMS a | Jump to subroutine |
| 0x6_ | INC Rn | Increment register |
| 0x7_ | ISZ Rn,a | Increment and skip if zero |
| 0x8_ | ADD Rn | Add register to accumulator |
| 0x9_ | SUB Rn | Subtract register |
| 0xA_ | LD Rn | Load register to accumulator |
| 0xB_ | XCH Rn | Exchange accumulator and register |
| 0xC_ | BBL n | Branch back and load |
| 0xD_ | LDM n | Load immediate |
| 0xE0-EF | I/O | RAM/ROM read/write |
| 0xF0-FD | Accum | Accumulator operations |

## Architecture

The Elixir simulator models the CPU as an immutable struct. Each instruction transforms the state into a new state — no mutation. This is a natural fit for hardware: a circuit is a pure function from (current state, input) → next state.

## Running Tests

```bash
mix test
```
