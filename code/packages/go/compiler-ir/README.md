# compiler-ir

A **general-purpose intermediate representation (IR)** for the AOT native
compiler pipeline. This package defines the IR types, opcodes, printer,
and parser — the shared vocabulary that all compiler frontends, optimizers,
and backends speak.

## Strict BF03 core profile

The additive `core-text-v1` API follows
[the language-neutral contract](../../../specs/BF03-core-profile.md).
`ValidateCoreProfile(program)` returns an error for unsupported data;
`PrintCoreProfile(program)` and `ParseCoreProfile(text)` exchange exactly the
canonical bytes. COMMENT operands use `IrText{Value: "note"}`, distinct from
`IrLabel`. Only value operands are admitted; nil and pointer operands fail
without panicking. Native nil/empty slices both represent empty vectors.

```go
program := ir.NewIrProgram("_start")
program.AddInstruction(ir.IrInstruction{
    Opcode: ir.OpComment, Operands: []ir.IrOperand{ir.IrText{Value: "note"}}, ID: -1,
})
text, err := ir.PrintCoreProfile(program) // checked canonical text
parsed, err := ir.ParseCoreProfile(text) // owned result, exact spelling only
ids, next, err := ir.CoreIDSequence(2147483647, 1)
// ids == []int64{2147483647}; next == 2147483648 (exhausted observation)
```

The checked suffix API fails atomically, returning nil values and unchanged
start; it does not modify the existing unchecked `IDGenerator`. Ordinary IDs
can be sparse/nonmonotone; only LABEL/COMMENT use -1. No reference resolution
or instruction execution occurs. Text is bounded to65536bytes, programs to
256instructions/32declarations, and all names/numbers/tags follow the profile.

Existing `Parse`/`Print` retain their broad debug behavior and zero-operand
padding, including Nib's colon-bearing `IrLabel` comments. The strict family
deliberately rejects those outside-profile payloads without rewriting them.
This adoption covers Go's data/text component, not source maps, optimizers,
native backends, runtime execution or the other14established language lanes.

Tests consume all50frozen cases/25operation roles through bounded transport,
with independent rosters, a digest and literal sparse-ID golden. Run the real
`BUILD` (`go test ./... -v -cover`), `go vet ./...` and `go build ./...`.
Coverage includes all shipped production files, including generated outcome
helpers; fixture-reader statements are test-only and earn no production credit.
Full corpus execution is validated on a 64-bit host. Checked integer admission
also passes focused 32-bit tests; the full corpus is not qualified on 32-bit
hosts because some semantic-negative values exceed the legacy native `int`.

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
