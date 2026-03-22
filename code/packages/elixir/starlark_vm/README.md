# Starlark VM (Elixir)

A complete Starlark bytecode interpreter built on GenericVM.

## What It Does

This package executes Starlark bytecode produced by the `starlark_ast_to_bytecode_compiler`. It provides all 46+ opcode handlers and 23 built-in functions, creating a fully functional Starlark runtime.

## Architecture

The VM plugs into the GenericVM framework by:
1. Registering handler functions for each Starlark opcode
2. Registering 23 built-in functions (len, range, sorted, etc.)
3. Providing factory functions for configured VM instances

## Key Types

- `StarlarkFunction` — compiled function objects with code, params, defaults
- `StarlarkIterator` — iterator wrapper for for-loop support
- `StarlarkResult` — execution result with variables, output, traces

## Usage

```elixir
alias CodingAdventures.StarlarkVm

# One-call execution
result = StarlarkVm.execute_starlark("x = 1 + 2\nprint(x)\n")
result.variables["x"]  #=> 3
result.output           #=> ["3"]

# Step by step
vm = StarlarkVm.create_starlark_vm()
code = StarlarkAstToBytecodeCompiler.compile_starlark("x = 42\n")
{traces, vm} = GenericVM.execute(vm, code)
```

## Built-in Functions (23)

Type: `type`, `bool`, `int`, `float`, `str`
Collections: `len`, `list`, `dict`, `tuple`, `range`, `sorted`, `reversed`, `enumerate`, `zip`
Math/Logic: `min`, `max`, `abs`, `all`, `any`
Utility: `repr`, `hasattr`, `getattr`
I/O: `print`

## Dependencies

- `coding_adventures_virtual_machine` — GenericVM framework
- `coding_adventures_bytecode_compiler` — GenericCompiler framework
- `coding_adventures_starlark_ast_to_bytecode_compiler` — Opcodes and compiler
