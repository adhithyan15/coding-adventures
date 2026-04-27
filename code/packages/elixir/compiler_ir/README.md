# compiler_ir (Elixir)

Elixir port of the general-purpose intermediate representation (IR) for the AOT native compiler pipeline.

## What is this?

This package defines the data types and operations for a general-purpose IR — the bridge between
language frontends (Brainfuck, BASIC, Lua) and machine-code backends (RISC-V, x86, ARM).

The IR is:
- **Linear**: flat list of instructions with labels and jumps (no basic blocks, no SSA)
- **Register-based**: infinite virtual registers (v0, v1, ...) mapped to physical ones by the backend
- **Target-independent**: backends consume IR without knowing the source language
- **Versioned**: the `.version` directive enables forward compatibility as opcodes are added

## How it fits in the stack

```
Source code
    ↓ (language frontend, e.g. brainfuck_ir_compiler)
IrProgram  ←── THIS PACKAGE
    ↓ (optimiser passes)
Optimised IrProgram
    ↓ (machine-code backend)
Binary / ELF / RISC-V assembly
```

## Usage

```elixir
alias CodingAdventures.CompilerIr.{
  IrProgram, IrInstruction, IrDataDecl,
  IrRegister, IrImmediate, IrLabel,
  IDGenerator, Printer, Parser
}

# Create a program
program = IrProgram.new("_start")
gen = IDGenerator.new()

# Add a data declaration
program = IrProgram.add_data(program, %IrDataDecl{label: "tape", size: 30000, init: 0})

# Emit instructions
{id0, gen} = IDGenerator.next(gen)
program = IrProgram.add_instruction(program, %IrInstruction{
  opcode: :label,
  operands: [%IrLabel{name: "_start"}],
  id: -1
})

{id1, gen} = IDGenerator.next(gen)
program = IrProgram.add_instruction(program, %IrInstruction{
  opcode: :load_addr,
  operands: [%IrRegister{index: 0}, %IrLabel{name: "tape"}],
  id: id1
})

{id2, _gen} = IDGenerator.next(gen)
program = IrProgram.add_instruction(program, %IrInstruction{
  opcode: :halt, operands: [], id: id2
})

# Print to text
text = Printer.print(program)

# Parse back (roundtrip)
{:ok, reparsed} = Parser.parse(text)
```

## Text Format

```
.version 1

.data tape 30000 0

.entry _start

_start:
  LOAD_ADDR   v0, tape          ; #0
  LOAD_IMM    v1, 0             ; #1
  HALT                          ; #2
```

## Modules

| Module | Purpose |
|--------|---------|
| `IrOp` | Opcode atom constants and name↔atom conversion |
| `IrRegister` | Virtual register operand (`v0`, `v1`, ...) |
| `IrImmediate` | Integer immediate operand |
| `IrLabel` | Named label operand |
| `IrInstruction` | A single IR instruction (opcode + operands + ID) |
| `IrDataDecl` | Data segment declaration (`.data label size init`) |
| `IrProgram` | A complete compiled program |
| `IDGenerator` | Monotonically increasing unique instruction IDs |
| `Printer` | Render `IrProgram` to IR text |
| `Parser` | Parse IR text back to `IrProgram` |

## Running tests

```bash
mix deps.get && mix test --cover
```
