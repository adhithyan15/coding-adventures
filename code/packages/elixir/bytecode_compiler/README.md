# Bytecode Compiler

**Layer 4 of the Computing Stack** — A pluggable AST-to-bytecode compiler framework for Elixir.

## What It Does

The `GenericCompiler` walks an Abstract Syntax Tree (AST) and emits bytecode instructions for a virtual machine. Instead of being hardcoded for one language, it uses a plugin system: you register handler functions for each AST node type, and the compiler dispatches to them automatically.

## How It Fits in the Stack

```
Source Code -> Lexer -> Parser -> [Bytecode Compiler] -> Virtual Machine
```

The lexer produces tokens. The parser arranges them into an AST. This compiler converts the AST into a `CodeObject` containing bytecode instructions, a constant pool, and a name table. The virtual machine then executes the `CodeObject`.

## Key Features

- **Pluggable handlers** — Register a function for each AST rule name
- **Immutable state** — Every operation returns a new compiler struct
- **Constant/name deduplication** — Pools stay compact automatically
- **Jump patching** — Emit placeholders, patch targets later
- **Scope tracking** — Local variable management for function compilation
- **Pass-through nodes** — Single-child wrapper nodes need no handler
- **Nested compilation** — Compile sub-trees into standalone CodeObjects

## Usage

```elixir
alias CodingAdventures.BytecodeCompiler.GenericCompiler

# Create a compiler and register handlers
compiler = GenericCompiler.new()

compiler = GenericCompiler.register_rule(compiler, "number", fn compiler, node ->
  token = hd(node.children)
  value = String.to_integer(token.value)
  {index, compiler} = GenericCompiler.add_constant(compiler, value)
  {_idx, compiler} = GenericCompiler.emit(compiler, 0x01, index)
  compiler
end)

# Compile an AST
ast = %{rule_name: "number", children: [%{type: "NUMBER", value: "42"}]}
{code_object, _compiler} = GenericCompiler.compile(compiler, ast)

# code_object.instructions => [LOAD_CONST 0, HALT]
# code_object.constants    => [42]
```

## Dependencies

- `coding_adventures_virtual_machine` — provides `Instruction` and `CodeObject` types
