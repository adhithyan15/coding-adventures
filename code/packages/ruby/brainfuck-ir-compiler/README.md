# coding_adventures_brainfuck_ir_compiler

Brainfuck AOT compiler frontend — translates a Brainfuck AST into target-independent IR.

## What it does

This gem is the Brainfuck-specific frontend of the AOT native compiler pipeline. It takes a Brainfuck AST (produced by `coding_adventures_brainfuck`) and emits:

1. An `IrProgram` (from `coding_adventures_compiler_ir`) — the compiled IR instructions
2. A `SourceMapChain` with segments 1 and 2 filled — for debugging and source-level error reporting

It does NOT know about RISC-V, ARM, ELF, or any specific machine target. That is the backend's job.

## Pipeline position

```
Source text (hello.bf)
    ↓  coding_adventures_brainfuck: Parser.parse
AST (CodingAdventures::Parser::ASTNode)
    ↓  coding_adventures_brainfuck_ir_compiler: BrainfuckIrCompiler.compile
IrProgram + SourceMapChain (segments 1+2)   ← this gem
    ↓  compiler-ir-optimizer (future)
Optimised IrProgram + SourceMapChain (segments 1+2+3)
    ↓  codegen-riscv (future)
ELF binary + complete SourceMapChain
```

## BuildConfig

Compilation is controlled by `BuildConfig` flags:

| Flag | Debug | Release | Description |
|------|-------|---------|-------------|
| `insert_bounds_checks` | true | false | Emit tape pointer range checks before `>` and `<` |
| `insert_debug_locs` | true | false | Emit COMMENT source-location markers |
| `mask_byte_arithmetic` | true | true | Emit AND_IMM 255 after `+` and `-` |
| `tape_size` | 30000 | 30000 | Number of tape cells |

## Usage

```ruby
require "coding_adventures_brainfuck"
require "coding_adventures_brainfuck_ir_compiler"

BIC = CodingAdventures::BrainfuckIrCompiler
IR  = CodingAdventures::CompilerIr

# Parse the Brainfuck source into an AST
ast = CodingAdventures::Brainfuck::Parser.parse("++[>+<-]>.")

# Compile with release flags
config = BIC::BuildConfig.release_config
result = BIC.compile(ast, "hello.bf", config)

# result.program is an IrProgram
text = IR::IrPrinter.print(result.program)
puts text

# result.source_map is a SourceMapChain with segments 1+2 filled
puts result.source_map.source_to_ast.entries.length  # number of source positions mapped
```

## Command-to-IR mapping

| Command | IR output |
|---------|-----------|
| `>` (RIGHT) | `ADD_IMM v1, v1, 1` |
| `<` (LEFT) | `ADD_IMM v1, v1, -1` |
| `+` (INC) | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
| `-` (DEC) | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
| `.` (OUTPUT) | `LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1` |
| `,` (INPUT) | `SYSCALL 2; STORE_BYTE v4, v0, v1` |
