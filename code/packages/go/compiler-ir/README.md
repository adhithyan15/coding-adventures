# compiler-ir

A **general-purpose intermediate representation (IR)** for the AOT native
compiler pipeline. This package defines the IR types, opcodes, printer,
and parser — the shared vocabulary that all compiler frontends, optimizers,
and backends speak.

## Design Philosophy

The IR is designed to serve **any compiled language**, not just Brainfuck.
The current v1 instruction set is the subset needed for Brainfuck. When
BASIC is added as the next frontend, new opcodes (MUL, DIV, float ops,
string primitives) will be appended. Existing opcodes never change
semantics — only new ones are added. This keeps all frontends and backends
forward-compatible.

## IR Characteristics

- **Linear** — no basic blocks, no SSA, no phi nodes
- **Register-based** — infinite virtual registers (v0, v1, ...)
- **Target-independent** — backends map to physical ISAs
- **Versioned** — `.version N` in text format (v1 = Brainfuck subset)

## Instruction Set (v1)

| Category | Instructions |
|----------|-------------|
| Constants | `LOAD_IMM`, `LOAD_ADDR` |
| Memory | `LOAD_BYTE`, `STORE_BYTE`, `LOAD_WORD`, `STORE_WORD` |
| Arithmetic | `ADD`, `ADD_IMM`, `SUB`, `AND`, `AND_IMM` |
| Comparison | `CMP_EQ`, `CMP_NE`, `CMP_LT`, `CMP_GT` |
| Control Flow | `LABEL`, `JUMP`, `BRANCH_Z`, `BRANCH_NZ`, `CALL`, `RET` |
| System | `SYSCALL`, `HALT` |
| Meta | `NOP`, `COMMENT` |

## Usage

```go
import ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"

// Build a program
gen := ir.NewIDGenerator()
program := ir.NewIrProgram("_start")
program.AddData(ir.IrDataDecl{Label: "tape", Size: 30000, Init: 0})
program.AddInstruction(ir.IrInstruction{
    Opcode:   ir.OpLoadAddr,
    Operands: []ir.IrOperand{ir.IrRegister{0}, ir.IrLabel{"tape"}},
    ID:       gen.Next(),
})

// Print to text
text := ir.Print(program)

// Parse from text
parsed, err := ir.Parse(text)

// Roundtrip invariant: parse(print(p)) ≡ p
```

## Text Format

```
.version 1

.data tape 30000 0

.entry _start

_start:
  LOAD_ADDR  v0, tape          ; #0
  LOAD_IMM   v1, 0             ; #1
  HALT                          ; #2
```

## Part Of

This package is part of the AOT native compiler pipeline (spec BF03).
See the [spec](../../../specs/BF03-aot-native-compiler.md) for the full
architecture.
