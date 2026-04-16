# brainfuck-ir-compiler

Brainfuck AOT compiler frontend: translates a Brainfuck AST into the general-purpose IR.

## Overview

`brainfuck-ir-compiler` is the Brainfuck-specific **frontend** of the AOT compiler pipeline. It:

- Takes a `GrammarASTNode` (produced by the `brainfuck` crate's parser)
- Emits IR instructions from `compiler-ir`
- Fills the first two segments of the `SourceMapChain` from `compiler-source-map`

It knows Brainfuck semantics (tape, cells, pointer, loops, I/O). It does NOT know about RISC-V, ARM, ELF, or any specific machine target.

## Pipeline position

```
Brainfuck source
       |
  [brainfuck::parser] → GrammarASTNode
       |
  [brainfuck-ir-compiler] ← THIS CRATE
       |
  IrProgram + SourceMapChain
       |
  [optimizer] (future)
       |
  [codegen-riscv] (future)
```

## Usage

```rust
use brainfuck_ir_compiler::{compile, release_config};
use brainfuck::parser::parse_brainfuck;

let ast = parse_brainfuck("++[>+<-].").unwrap();
let result = compile(&ast, "hello.bf", release_config()).unwrap();

// result.program  — the IrProgram
// result.source_map — SourceMapChain with SourceToAst + AstToIr filled
```

## Compilation mapping

| Command | IR output |
|---------|-----------|
| `>`     | `ADD_IMM v1, v1, 1` |
| `<`     | `ADD_IMM v1, v1, -1` |
| `+`     | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
| `-`     | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
| `.`     | `LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1` |
| `,`     | `SYSCALL 2; STORE_BYTE v4, v0, v1` |
| `[`/`]` | `LABEL loop_N_start; LOAD_BYTE; BRANCH_Z loop_N_end; ...; JUMP loop_N_start; LABEL loop_N_end` |

## Build modes

```rust
use brainfuck_ir_compiler::{debug_config, release_config, BuildConfig};

// Debug: bounds checks + debug locations + byte masking
let dbg = debug_config();

// Release: byte masking only (no bounds checks)
let rel = release_config();

// Custom
let mut custom = release_config();
custom.tape_size = 1000;
```

| Preset            | Bounds checks | Debug locs | Byte masking | Tape size |
|-------------------|:---:|:---:|:---:|------:|
| `debug_config()`  | ON  | ON  | ON  | 30000 |
| `release_config()`| OFF | OFF | ON  | 30000 |

## Register layout

| Register | Purpose |
|----------|---------|
| v0       | tape base address |
| v1       | tape pointer offset (current cell index) |
| v2       | temporary for cell values |
| v3       | temporary for bounds checks |
| v4       | syscall argument/return |
| v5       | `tape_size - 1` (upper bounds check) |
| v6       | constant 0 (lower bounds check) |
