# brainfuck-ir-compiler (Python)

Brainfuck-specific frontend of the AOT compiler pipeline. Translates a
Brainfuck AST into the general-purpose IR defined by `compiler-ir`.

## What It Does

This package is the "language-specific" part of the compiler. It knows
Brainfuck semantics (tape, cells, pointer, loops, I/O) and translates them
into target-independent IR instructions. It does NOT know about RISC-V, ARM,
ELF, or any specific machine target.

The compiler produces two outputs:
1. An `IrProgram` with the compiled IR instructions
2. A `SourceMapChain` with `SourceToAst` and `AstToIr` segments filled in

## Usage

```python
from brainfuck import parse_brainfuck
from brainfuck_ir_compiler import compile_brainfuck, release_config
from compiler_ir import print_ir

ast = parse_brainfuck("+.")
result = compile_brainfuck(ast, "hello.bf", release_config())

# Print the IR
print(print_ir(result.program))
```

## Command → IR Mapping

| Command  | IR Output                                          |
|----------|----------------------------------------------------|
| `>` RIGHT | `ADD_IMM v1, v1, 1`                              |
| `<` LEFT  | `ADD_IMM v1, v1, -1`                             |
| `+` INC   | `LOAD_BYTE v2, v0, v1` + `ADD_IMM` + `AND_IMM` + `STORE_BYTE` |
| `-` DEC   | Same as INC but with delta -1                     |
| `.` OUTPUT | `LOAD_BYTE` + `ADD_IMM v4, v2, 0` + `SYSCALL 1` |
| `,` INPUT  | `SYSCALL 2` + `STORE_BYTE v4, v0, v1`           |

## Build Modes

```python
# Debug: bounds checks + debug locs + byte masking
from brainfuck_ir_compiler import debug_config

# Release: no bounds checks, byte masking only
from brainfuck_ir_compiler import release_config
```

## Dependencies

- `coding-adventures-brainfuck` (for the parser)
- `coding-adventures-compiler-ir` (for IR types)
- `coding-adventures-compiler-source-map` (for the source map chain)

## Tests

```bash
cd code/packages/python/brainfuck-ir-compiler
mise exec -- uv run pytest tests/ --cov=src --cov-report=term-missing -v
```
