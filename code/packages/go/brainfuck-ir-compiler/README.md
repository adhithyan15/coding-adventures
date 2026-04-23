# brainfuck-ir-compiler

A Brainfuck-specific frontend that compiles Brainfuck ASTs into the
general-purpose intermediate representation (IR) defined by `compiler-ir`.

## What It Does

Takes a Brainfuck AST (from the parser, spec BF01) and produces:

1. An `IrProgram` containing target-independent IR instructions
2. Source map segments (`SourceToAst` + `AstToIr`) for debugging

The compiler knows Brainfuck semantics — tape, cells, pointer, loops, I/O —
and translates them into IR. It does **not** know about RISC-V, ARM, ELF,
or any specific machine target.

## Build Configurations

Build modes are composable flags, not a fixed enum:

| Flag | Debug | Release | Effect |
|------|-------|---------|--------|
| `InsertBoundsChecks` | ✓ | ✗ | Trap on tape pointer out-of-bounds |
| `InsertDebugLocs` | ✓ | ✗ | Emit source location comments |
| `MaskByteArithmetic` | ✓ | ✓ | AND 0xFF after cell mutation |
| `TapeSize` | 30000 | 30000 | Configurable tape length |

## Compilation Mapping

| BF Command | IR Instructions |
|------------|----------------|
| `>` (RIGHT) | `ADD_IMM v1, v1, 1` |
| `<` (LEFT) | `ADD_IMM v1, v1, -1` |
| `+` (INC) | `LOAD_BYTE → ADD_IMM → AND_IMM → STORE_BYTE` |
| `-` (DEC) | `LOAD_BYTE → ADD_IMM(-1) → AND_IMM → STORE_BYTE` |
| `.` (OUTPUT) | `LOAD_BYTE → ADD_IMM 0 (copy to arg) → SYSCALL 1` |
| `,` (INPUT) | `SYSCALL 2 → STORE_BYTE` |
| `[` (LOOP) | `LABEL → LOAD_BYTE → BRANCH_Z` |
| `]` (END) | `JUMP → LABEL` |

## Usage

```go
import (
    "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck"
    bfir "github.com/adhithyan15/coding-adventures/code/packages/go/brainfuck-ir-compiler"
    ir "github.com/adhithyan15/coding-adventures/code/packages/go/compiler-ir"
)

// Parse Brainfuck source
ast, _ := brainfuck.ParseBrainfuck("++[>+<-].")

// Compile to IR
result, _ := bfir.Compile(ast, "hello.bf", bfir.DebugConfig())

// Use the IR program
text := ir.Print(result.Program)
fmt.Println(text)

// Use the source map
for _, entry := range result.SourceMap.SourceToAst.Entries {
    fmt.Printf("%s → AST node %d\n", entry.Pos, entry.AstNodeID)
}
```

## Register Allocation

Brainfuck uses a fixed register mapping (no allocator needed):

| Register | Purpose |
|----------|---------|
| v0 | Tape base address |
| v1 | Tape pointer (cell index) |
| v2 | Temporary (cell values) |
| v3 | Temporary (bounds checks) |
| v4 | Syscall argument |
| v5 | Max pointer (debug only) |
| v6 | Zero constant (debug only) |

## Part Of

This package is part of the AOT native compiler pipeline (spec BF03).
See the [spec](../../../specs/BF03-aot-native-compiler.md) for the full
architecture.
