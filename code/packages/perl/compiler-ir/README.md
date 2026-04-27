# CodingAdventures::CompilerIr

General-purpose intermediate representation (IR) for the coding-adventures AOT compiler pipeline. This is a Perl port of the Go package at `code/packages/go/compiler-ir/`.

## What is this?

An IR sits between the source language and target machine code:

```
Source (Brainfuck)
     ↓  [brainfuck-ir-compiler]
   IR  ← this package
     ↓  [compiler-ir-optimizer]
   IR (optimised)
     ↓  [codegen-riscv]
Machine code (ELF binary)
```

By targeting a common IR, multiple frontends (Brainfuck, BASIC, ...) and multiple backends (RISC-V, ARM, x86, ...) avoid an N×M explosion of compiler passes.

## IR Properties

- **Linear**: no basic blocks, no SSA, no phi nodes
- **Register-based**: infinite virtual registers (v0, v1, ...)
- **Target-independent**: backends map IR to physical ISA
- **Versioned**: `.version` directive in text format (v1 = Brainfuck subset)

## Modules

| Module | Purpose |
|--------|---------|
| `IrOp` | 25 opcode constants and name↔integer table |
| `IrRegister` | Virtual register operand (`v0`, `v1`, ...) |
| `IrImmediate` | Literal integer operand (`42`, `-1`, `255`) |
| `IrLabel` | Named label operand (`_start`, `loop_0_end`, `tape`) |
| `IrInstruction` | One IR instruction: opcode + operands + unique ID |
| `IrDataDecl` | Data segment declaration (`.data tape 30000 0`) |
| `IrProgram` | Complete IR program: instructions + data + entry label |
| `IDGenerator` | Monotonic unique instruction ID counter |
| `Printer` | `IrProgram` → canonical text |
| `Parser` | Canonical text → `IrProgram` |

## Opcode Summary

| Category | Opcodes |
|----------|---------|
| Constants | `LOAD_IMM`, `LOAD_ADDR` |
| Memory | `LOAD_BYTE`, `STORE_BYTE`, `LOAD_WORD`, `STORE_WORD` |
| Arithmetic | `ADD`, `ADD_IMM`, `SUB`, `AND`, `AND_IMM` |
| Comparison | `CMP_EQ`, `CMP_NE`, `CMP_LT`, `CMP_GT` |
| Control Flow | `LABEL`, `JUMP`, `BRANCH_Z`, `BRANCH_NZ`, `CALL`, `RET` |
| System | `SYSCALL`, `HALT` |
| Meta | `NOP`, `COMMENT` |

## Usage

```perl
use CodingAdventures::CompilerIr qw(print_ir parse_ir);
use CodingAdventures::CompilerIr::IrOp;
use CodingAdventures::CompilerIr::IrProgram;
use CodingAdventures::CompilerIr::IrInstruction;
use CodingAdventures::CompilerIr::IrRegister;
use CodingAdventures::CompilerIr::IrImmediate;
use CodingAdventures::CompilerIr::IDGenerator;

my $gen  = CodingAdventures::CompilerIr::IDGenerator->new;
my $prog = CodingAdventures::CompilerIr::IrProgram->new('_start');

$prog->add_instruction(
    CodingAdventures::CompilerIr::IrInstruction->new(
        opcode   => CodingAdventures::CompilerIr::IrOp::HALT,
        operands => [],
        id       => $gen->next,
    )
);

print print_ir($prog);
# .version 1
#
# .entry _start
#   HALT          ; #0
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

## Dependencies

No runtime dependencies. Test dependency: `Test2::V0`.

## Running Tests

```bash
prove -l -v t/
```
