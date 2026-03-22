# Starlark Compiler

Compiles Starlark ASTs into bytecode using the pluggable GenericCompiler framework.

## What Is This?

This package translates the abstract syntax tree produced by `starlark-parser` into a flat sequence of bytecode instructions that the `starlark-vm` can execute. It is built on the `GenericCompiler` from the `bytecode-compiler` package, registering handlers for all 55 Starlark grammar rules.

The compiler produces `CodeObject` instances (from the `virtual-machine` package) containing instructions, a constants pool, and a names pool.

## What Is Starlark?

Starlark is a deterministic subset of Python designed by Google for BUILD files (Bazel, Buck). It supports `def`, `for`, `if/elif/else`, list/dict comprehensions, and lambda expressions, but intentionally removes `while`, `class`, `try/except`, `import`, and recursion to guarantee termination.

## How It Fits in the Stack

```
Starlark source code
    |
    v
starlark_lexer.tokenize_starlark()     -- tokenizes source
    |
    v
starlark_parser.parse_starlark()       -- produces AST
    |
    v
starlark_ast_to_bytecode_compiler.compile_starlark()   -- [this package] AST -> bytecode
    |
    v
CodeObject (instructions + constants + names)
    |
    v
starlark_vm.execute_starlark()         -- executes bytecode
```

## Key Features

- **~50 opcodes** organized by category via the `Op` enum: stack operations, arithmetic, comparison, variables, control flow, functions, data structures, iteration, and more.
- **All 55 grammar rules** handled: every Starlark AST node type has a corresponding compiler handler that emits the correct bytecode sequence.
- **Operator maps**: `BINARY_OP_MAP`, `COMPARE_OP_MAP`, and `AUGMENTED_ASSIGN_MAP` translate AST operator tokens to their bytecode equivalents.
- **Pluggable architecture**: built on `GenericCompiler`, so the same compilation framework can be reused for other languages by registering different rule handlers.
- **Literate programming style**: source code includes inline explanations, truth tables, and examples to teach compiler construction.

## Usage

```python
from starlark_ast_to_bytecode_compiler import compile_starlark, Op

# One-call compilation from source to bytecode
code = compile_starlark('x = 1 + 2\n')
print(code.instructions)  # [LOAD_CONST, LOAD_CONST, BINARY_ADD, STORE_NAME, ...]
print(code.constants)      # [1, 2]
print(code.names)          # ['x']

# Or create a reusable compiler instance
from starlark_ast_to_bytecode_compiler import create_starlark_ast_to_bytecode_compiler

compiler = create_starlark_ast_to_bytecode_compiler()
code = compiler.compile('def add(a, b):\n    return a + b\n')

# Inspect the opcode enum
print(list(Op))  # All ~50 opcodes
```

## Installation

```bash
pip install coding-adventures-starlark-ast-to-bytecode-compiler
```

## Dependencies

- `coding-adventures-bytecode-compiler` -- provides `GenericCompiler` framework
- `coding-adventures-virtual-machine` -- provides `CodeObject` for bytecode output
- `coding-adventures-starlark-parser` -- produces the AST this compiler consumes
- `coding-adventures-starlark-lexer` -- tokenizes Starlark source code
