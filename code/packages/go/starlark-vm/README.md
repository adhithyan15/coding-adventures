# starlark-vm

A Starlark virtual machine built on the GenericVM from `virtual-machine`. This package provides all 59 opcode handlers and 23 built-in functions needed to execute compiled Starlark bytecode.

## Architecture

```
Source Code → Lexer → Parser → AST → Compiler → CodeObject → THIS PACKAGE → Result
```

The `starlark-vm` package sits at the end of the pipeline. It takes a `CodeObject` (produced by `starlark-ast-to-bytecode-compiler`) and executes it on a configured `GenericVM`.

## How It Works

The package has four main components:

1. **Types** (`types.go`) — Runtime types: `StarlarkFunction`, `StarlarkIterator`, `StarlarkResult`
2. **Handlers** (`handlers.go`) — 59 opcode handlers covering stack ops, arithmetic, comparisons, control flow, functions, collections, iteration, and more
3. **Builtins** (`builtins.go`) — 23 built-in functions: `len`, `range`, `sorted`, `type`, `print`, `min`, `max`, `abs`, `all`, `any`, `enumerate`, `zip`, `reversed`, `repr`, `hasattr`, `getattr`, `bool`, `int`, `float`, `str`, `list`, `dict`, `tuple`
4. **VM Factory** (`vm.go`) — `CreateStarlarkVM()` and `ExecuteStarlark()` convenience functions

## Usage

### Quick Execution

```go
result, err := starlarkvm.ExecuteStarlark(`
x = 1 + 2
print(x)
`)
// result.Variables["x"] == 3
// result.Output == ["3"]
```

### Custom VM

```go
v := starlarkvm.CreateStarlarkVM(500) // max recursion depth 500
code, _ := starlarkcompiler.CompileStarlark(source)
traces := v.Execute(code)
```

## Dependencies

- `virtual-machine` — GenericVM, CodeObject, Instruction types
- `starlark-ast-to-bytecode-compiler` — CompileStarlark function, opcode constants

## Supported Opcodes

All 59 opcodes from the Starlark compiler are handled:

| Category | Opcodes |
|----------|---------|
| Stack | LOAD_CONST, POP, DUP, LOAD_NONE, LOAD_TRUE, LOAD_FALSE |
| Variables | STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL, STORE_CLOSURE, LOAD_CLOSURE |
| Arithmetic | ADD, SUB, MUL, DIV, FLOOR_DIV, MOD, POWER, NEGATE |
| Bitwise | BIT_AND, BIT_OR, BIT_XOR, BIT_NOT, LEFT_SHIFT, RIGHT_SHIFT |
| Comparison | CMP_EQ, CMP_NE, CMP_LT, CMP_GT, CMP_LE, CMP_GE, CMP_IN, CMP_NOT_IN, NOT |
| Control | JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE, JUMP_IF_FALSE_OR_POP, JUMP_IF_TRUE_OR_POP, BREAK, CONTINUE |
| Functions | MAKE_FUNCTION, CALL_FUNCTION, CALL_FUNCTION_KW, RETURN_VALUE |
| Collections | BUILD_LIST, BUILD_DICT, BUILD_TUPLE, LIST_APPEND, DICT_SET |
| Subscript | LOAD_SUBSCRIPT, STORE_SUBSCRIPT, LOAD_ATTR, STORE_ATTR, LOAD_SLICE |
| Iteration | GET_ITER, FOR_ITER, UNPACK_SEQUENCE |
| Modules | LOAD_MODULE, IMPORT_FROM |
| Output | PRINT_VALUE |
| Halt | HALT |
