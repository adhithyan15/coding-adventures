# starlark-ast-to-bytecode-compiler

Compiles Starlark Abstract Syntax Trees into bytecode instructions for the virtual machine.

## Position in the Stack

This package is the bridge between parsing and execution in the Starlark pipeline:

```
Source Code
    |
    v
starlark-lexer      (tokenization)
    |
    v
starlark-parser     (AST construction)
    |
    v
THIS PACKAGE        (bytecode compilation)
    |
    v
virtual-machine     (execution)
```

## What It Does

The compiler walks the AST produced by `starlark-parser` and emits a sequence of bytecode instructions packaged as a `CodeObject`. The `CodeObject` contains:

- **Instructions** — a flat array of opcodes with operands
- **Constants** — literal values (integers, strings, floats, nested CodeObjects)
- **Names** — variable, function, and attribute names

## Supported Language Features

- Variable assignment and references
- Arithmetic: `+`, `-`, `*`, `/`, `//`, `%`, `**`
- Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`
- Comparisons: `==`, `!=`, `<`, `>`, `<=`, `>=`, `in`, `not in`
- Boolean: `and`, `or`, `not` (with short-circuit evaluation)
- Control flow: `if`/`elif`/`else`, `for` loops, `break`, `continue`
- Functions: `def` with parameters and defaults, `return`, calls
- Collections: list `[]`, dict `{}`, tuple `()`
- Attribute access: `obj.attr`
- Subscript: `lst[0]`
- Load statement: `load("module", "symbol")`
- Lambda expressions
- Ternary conditionals: `x if cond else y`

## Usage

```go
import starlarkcompiler "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-ast-to-bytecode-compiler"

// One-shot: source code to bytecode
code, err := starlarkcompiler.CompileStarlark("x = 1 + 2\n")
// code.Instructions = [LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT]
// code.Constants = [1, 2]
// code.Names = ["x"]

// Debug: disassemble to readable form
fmt.Println(starlarkcompiler.Disassemble(code))
```

## Opcodes

This package defines 46 bytecode opcodes organized by category:

| Range       | Category                    | Examples                    |
|-------------|-----------------------------|-----------------------------|
| 0x01-0x06   | Stack manipulation          | LOAD_CONST, LOAD_NONE       |
| 0x10-0x15   | Variables                   | STORE_NAME, LOAD_LOCAL       |
| 0x20-0x2D   | Arithmetic & bitwise        | ADD, FLOOR_DIV, BIT_AND      |
| 0x30-0x38   | Comparisons & logic         | CMP_EQ, CMP_IN, NOT          |
| 0x40-0x46   | Control flow                | JUMP, FOR_ITER, BREAK        |
| 0x50-0x53   | Functions                   | MAKE_FUNCTION, CALL_FUNCTION |
| 0x60-0x64   | Collections                 | BUILD_LIST, BUILD_DICT       |
| 0x70-0x74   | Attribute & subscript       | LOAD_ATTR, LOAD_SUBSCRIPT    |
| 0x80-0x82   | Iteration                   | GET_ITER, FOR_ITER           |
| 0x90-0x91   | Modules                     | LOAD_MODULE, IMPORT_FROM     |

## Dependencies

- `starlark-parser` — provides the AST
- `virtual-machine` — provides OpCode, Instruction, and CodeObject types
- `lexer` — provides Token types for AST traversal
- `parser` — provides ASTNode type
