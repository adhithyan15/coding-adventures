# Starlark VM

Executes Starlark bytecode on the pluggable GenericVM framework.

## What Is This?

This package provides a complete Starlark runtime. It takes the bytecode produced by `starlark-compiler` and executes it on a `GenericVM` from the `virtual-machine` package, with ~50 opcode handlers and ~25 built-in functions that implement Starlark's semantics.

The VM enforces Starlark's restrictions (no recursion, deterministic evaluation) and provides Starlark-specific type behavior (int/float promotion, string concatenation, truthiness rules).

## What Is Starlark?

Starlark is a deterministic subset of Python designed by Google for BUILD files (Bazel, Buck). It supports `def`, `for`, `if/elif/else`, list/dict comprehensions, and lambda expressions, but intentionally removes `while`, `class`, `try/except`, `import`, and recursion to guarantee termination.

## How It Fits in the Stack

```
Starlark source code
    |
    v
starlark_lexer.tokenize_starlark()        -- tokenizes source
    |
    v
starlark_parser.parse_starlark()           -- produces AST
    |
    v
starlark_compiler.compile_starlark()       -- AST -> bytecode
    |
    v
starlark_vm.execute_starlark()             -- [this package] executes bytecode
    |
    v
StarlarkResult (variables, output, traces)
```

## Key Features

- **~50 opcode handlers**: every opcode emitted by the Starlark compiler has a corresponding handler that manipulates the VM stack, frames, and variables.
- **~25 built-in functions**: `len`, `range`, `sorted`, `reversed`, `enumerate`, `zip`, `map`, `filter`, `min`, `max`, `sum`, `abs`, `any`, `all`, `bool`, `int`, `float`, `str`, `list`, `dict`, `tuple`, `type`, `print`, `repr`, `hasattr`, and more.
- **Starlark type semantics**:
  - Int/float promotion in arithmetic (int + float = float)
  - String concatenation and repetition (`"a" + "b"`, `"a" * 3`)
  - Truthiness rules (empty collections are falsy, zero is falsy)
  - Immutable tuples, mutable lists and dicts
- **Iterator protocol**: `GET_ITER` and `FOR_ITER` opcodes support `for` loops over lists, dicts, ranges, strings, and other iterables via `StarlarkIterator`.
- **Function support**: `StarlarkFunction` wraps compiled `CodeObject` instances with parameter metadata for function calls.
- **Pluggable architecture**: built on `GenericVM`, so the same VM framework can be reused for other languages by registering different handlers.
- **Literate programming style**: source code includes inline explanations and examples to teach virtual machine construction.

## Usage

```python
from starlark_vm import execute_starlark, StarlarkResult

# One-call execution from source to result
result = execute_starlark('x = 1 + 2\n')
print(result.variables['x'])  # 3
print(result.output)           # '' (no print statements)

# Execute a function definition and call
result = execute_starlark('''
def greet(name):
    return "Hello, " + name + "!"

message = greet("World")
''')
print(result.variables['message'])  # 'Hello, World!'

# Use built-in functions
result = execute_starlark('''
numbers = [3, 1, 4, 1, 5]
total = sum(numbers)
biggest = max(numbers)
ordered = sorted(numbers)
''')
print(result.variables['total'])    # 14
print(result.variables['biggest'])  # 5
print(result.variables['ordered'])  # [1, 1, 3, 4, 5]

# Or create a reusable VM instance
from starlark_vm import create_starlark_vm

vm = create_starlark_vm()
```

## Installation

```bash
pip install coding-adventures-starlark-vm
```

## Dependencies

- `coding-adventures-virtual-machine` -- provides `GenericVM` framework
- `coding-adventures-starlark-compiler` -- provides `Op` enum and bytecode compilation
