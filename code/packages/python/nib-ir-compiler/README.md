# nib-ir-compiler

**Stage 4 of the Nib compiler pipeline** — translates a typed Nib AST into
general-purpose IR (`IrProgram` from `compiler-ir`).

## What is Nib?

Nib is a statically-typed toy language designed to compile to Intel 4004
machine code. The name comes from "nibble" — the 4-bit native word size of
the Intel 4004 (the world's first commercial microprocessor, 1971). Nib has
four types: `u4`, `u8`, `bcd`, and `bool`.

## Pipeline Position

```
Source text
    → nib-lexer          (characters → tokens)
    → nib-parser         (tokens → untyped ASTNode tree)
    → nib-type-checker   (untyped AST → typed AST)
    → nib-ir-compiler    (typed AST → IrProgram)   ← this package
    → [backend-validator]  (IrProgram → validated IR for specific ISA)
    → [code-generator]     (validated IR → Intel 4004 machine code)
```

## Installation

```bash
uv pip install coding-adventures-nib-ir-compiler
```

For local development (from the repo root):

```bash
cd code/packages/python/nib-ir-compiler
uv pip install -e ".[dev]"
```

## Quick Start

```python
from nib_parser import parse_nib
from nib_type_checker import check
from nib_ir_compiler import compile_nib

source = """
    fn add(a: u4, b: u4) -> u4 {
        return a +% b;
    }
    fn main() {
        let result: u4 = add(3, 4);
    }
"""

ast = parse_nib(source)
result = check(ast)
assert result.ok

compiled = compile_nib(result.typed_ast)
program = compiled.program

# Inspect the IR:
from compiler_ir import print_ir
print(print_ir(program))
```

## Public API

### `compile_nib(typed_ast, config?) -> CompileResult`

The main entry point. Takes a typed AST (annotated by the type checker)
and returns a `CompileResult`.

```python
from nib_ir_compiler import compile_nib, debug_config, release_config

compiled = compile_nib(typed_ast)                  # default: debug comments on
compiled = compile_nib(typed_ast, debug_config())  # debug: COMMENT instructions
compiled = compile_nib(typed_ast, release_config()) # release: no COMMENT instructions
```

### `CompileResult`

```python
@dataclass
class CompileResult:
    program: IrProgram             # the compiled IR
    source_map: SourceMapChain | None  # None in v1
```

### `BuildConfig`

```python
@dataclass
class BuildConfig:
    insert_debug_comments: bool = True

debug_config()   # → BuildConfig(insert_debug_comments=True)
release_config() # → BuildConfig(insert_debug_comments=False)
```

## Virtual Register Layout

Nib v1 uses a fixed register allocation:

| Register | Role |
|----------|------|
| `v0`     | Constant zero (preloaded to 0 at `_start`) |
| `v1`     | Scratch / expression result / return value |
| `v2`+    | Named variables (locals, params), allocated in order |

## Calling Convention

| Item | Detail |
|------|--------|
| Arguments | Passed in `v2`, `v3`, `v4`, ... (caller-save) |
| Return value | Left in `v1` by callee before `RET` |
| Callee regs | Fresh allocation from `v2` (no shared state) |

## IR Emission Summary

### Program Entry Point

Every compiled program begins:

```
LABEL     _start
LOAD_IMM  v0, 0          ; initialize the zero-constant register
CALL      _fn_main       ; invoke main (if declared)
HALT                     ; terminate
```

### Declarations

| Source | IR |
|--------|----|
| `const NAME: T = val` | No IR (inlined at use sites) |
| `static NAME: T = val` | `IrDataDecl(label=NAME, size=T.size_bytes, init=val)` |
| `fn NAME(params) -> T { body }` | `LABEL _fn_NAME` + body + `RET` |

### Statements

| Source | IR |
|--------|----|
| `let x: T = expr` | compile expr into v1, `ADD_IMM vN, v1, 0` |
| `x = expr` | compile expr into v1, `ADD_IMM vN, v1, 0` |
| `return expr` | compile expr into v1, `RET` |
| `for i: T in s..e { }` | `LOAD_IMM vI, s` + `LABEL loop_K_start` + cond + body + `ADD_IMM + JUMP` + `LABEL loop_K_end` |
| `if cond { } else { }` | `BRANCH_Z vC, else_K` + then + `JUMP end_K` + `LABEL else_K` + else + `LABEL end_K` |

### Arithmetic

| Operator | IR | Notes |
|----------|-----|-------|
| `+%` (u4) | `ADD vT, vA, vB; AND_IMM vT, vT, 15` | Nibble mask |
| `+%` (u8/bcd) | `ADD vT, vA, vB; AND_IMM vT, vT, 255` | Byte mask |
| `-` | `SUB vT, vA, vB` | |
| `==` | `CMP_EQ vT, vA, vB` | |
| `!=` | `CMP_NE vT, vA, vB` | |
| `<` | `CMP_LT vT, vA, vB` | |
| `>` | `CMP_GT vT, vA, vB` | |
| `<=` | `CMP_GT vT, vB, vA` | Swap operands |
| `>=` | `CMP_LT vT, vB, vA` | Swap operands |
| `&&` | `AND vT, vA, vB` | |
| `\|\|` | `ADD vT, vA, vB; CMP_NE vT, vT, v0` | |
| `!` | `CMP_EQ vT, vA, v0` | |
| `&` | `AND vT, vA, vB` | |

## Running Tests

```bash
cd code/packages/python/nib-ir-compiler
uv run pytest
```

Or with coverage:

```bash
uv run pytest --cov=nib_ir_compiler --cov-report=html
```

## Dependencies

- `coding-adventures-compiler-ir` — `IrProgram`, `IrInstruction`, `IrOp`, etc.
- `coding-adventures-compiler-source-map` — `SourceMapChain`
- `coding-adventures-nib-parser` — `parse_nib()`
- `coding-adventures-nib-type-checker` — `check()`, `NibType`
- `coding-adventures-lang-parser` — `ASTNode`
- `coding-adventures-lexer` — `Token`

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
