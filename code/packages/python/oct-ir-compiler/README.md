# coding-adventures-oct-ir-compiler

Lowers typed Oct ASTs into general-purpose IR — the fourth stage of the Oct compiler pipeline.

## What Is This?

This package is the **fourth stage** of the Oct compiler pipeline:

```
Source text
    → oct-lexer         (characters → tokens)
    → oct-parser        (tokens → untyped ASTNode tree)
    → oct-type-checker  (untyped AST → typed AST)
    → oct-ir-compiler   (typed AST → IrProgram)   ← this package
    → intel-8008-ir-validator
    → ir-to-intel-8008-compiler
    → intel-8008-assembler
    → intel-8008-packager
```

Oct is a statically-typed systems programming language that compiles to Intel 8008 machine
code. This package lowers the type-annotated AST into target-independent IR (`IrProgram`
from `compiler-ir`), which the backend packages then lower further to 8008 assembly and
binary.

## Virtual Register Layout

Oct v1 uses a fixed virtual register allocation:

| Register | Role                                    |
|----------|-----------------------------------------|
| `v0`     | Constant zero (preloaded at `_start`)   |
| `v1`     | Scratch / expression temp / return value|
| `v2`     | First argument / first local (maps → B) |
| `v3`     | Second argument / second local (→ C)    |
| `v4`     | Third argument / third local (→ D)      |
| `v5`     | Fourth argument / fourth local (→ E)    |

## Calling Convention

- **Arguments**: placed in `v2`, `v3`, `v4`, `v5` (left to right)
- **Return value**: in `v1` (the accumulator)
- **Callee registers**: fresh `v2+` allocation per function

## Usage

```python
from oct_parser import parse_oct
from oct_type_checker import check_oct
from oct_ir_compiler import compile_oct

source = '''
    static THRESHOLD: u8 = 128;

    fn process(val: u8) -> bool {
        return val > THRESHOLD;
    }

    fn main() {
        let data: u8 = in(0);
        let high: bool = process(data);
        if high {
            out(1, data);
        }
    }
'''

ast = parse_oct(source)
result = check_oct(ast)
assert result.ok

compiled = compile_oct(result.typed_ast)
prog = compiled.program
# prog.entry_label == "_start"
# prog.data — IrDataDecl entries for static variables
# prog.instructions — the IR instruction stream
```

## IR Mapping

| Oct construct | IR opcodes |
|---------------|-----------|
| `let x: u8 = v` (literal) | `LOAD_IMM` |
| `let x: u8 = y` (copy) | `ADD_IMM 0` |
| `static x: u8` read | `LOAD_ADDR` + `LOAD_BYTE` |
| `static x = val` write | `LOAD_ADDR` + `STORE_BYTE` |
| `a + b` | `ADD` |
| `a - b` | `SUB` |
| `a & b` | `AND` |
| `a \| b` | `OR` |
| `a ^ b` | `XOR` |
| `~a` | `NOT` |
| `==`, `!=`, `<`, `>` | `CMP_EQ`, `CMP_NE`, `CMP_LT`, `CMP_GT` |
| `<=` | `CMP_GT(b, a)` (operand swap) |
| `>=` | `CMP_LT(b, a)` (operand swap) |
| `!a` | `CMP_EQ(a, v0)` |
| `a && b` | `AND` |
| `a \|\| b` | `ADD` + `CMP_NE` |
| `if cond` | `BRANCH_Z` / `JUMP` |
| `while cond` | `BRANCH_Z` (top check) + `JUMP` |
| `loop` | `JUMP` (unconditional) |
| `break` | `JUMP` to loop end |
| `fn call` | `CALL _fn_NAME` |
| `return v` | `RET` |
| `in(PORT)` | `SYSCALL 20+PORT` |
| `out(PORT, val)` | `SYSCALL 40+PORT` |
| `adc(a, b)` | `SYSCALL 3` |
| `sbb(a, b)` | `SYSCALL 4` |
| `rlc(a)` | `SYSCALL 11` |
| `rrc(a)` | `SYSCALL 12` |
| `ral(a)` | `SYSCALL 13` |
| `rar(a)` | `SYSCALL 14` |
| `carry()` | `SYSCALL 15` |
| `parity(a)` | `SYSCALL 16` |
| end of main | `HALT` |

## Project Structure

```
oct-ir-compiler/
├── src/
│   └── oct_ir_compiler/
│       ├── __init__.py      # Public API: OctCompileResult, compile_oct
│       └── compiler.py      # Full implementation (~900 lines, literate)
├── tests/
│   └── test_oct_ir_compiler.py   # ~250 test cases across 20 test classes
├── BUILD                    # Linux/macOS build script
├── BUILD_windows            # Windows build script
└── pyproject.toml
```

## How It Fits in the Stack

```
Layer 0: graph, directed-graph        — graph primitives
Layer 1: grammar-tools, lexer, parser — generic parsing infrastructure
Layer 2: state-machine                — automata
Layer 3: oct-lexer                    — Oct tokenizer
Layer 4: oct-parser                   — Oct grammar → ASTNode
Layer 5: type-checker-protocol        — TypeChecker protocol
Layer 6: oct-type-checker             — Oct type checker
Layer 7: compiler-ir                  — IrProgram, IrInstruction, IrOp
Layer 8: oct-ir-compiler  ← HERE      — Oct typed AST → IrProgram
Layer 9: intel-8008-ir-validator (next)
```

## Building and Testing

```bash
./BUILD          # on Linux/macOS
BUILD_windows    # on Windows
```

Creates a `.venv`, installs all dependencies from local source, and runs `pytest` with coverage.
