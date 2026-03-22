# Starlark AST-to-Bytecode Compiler (Elixir)

Compiles Starlark ASTs into bytecode for the Starlark VM.

## What It Does

This package is the bridge between the Starlark parser and the Starlark VM. It takes the Abstract Syntax Tree (AST) produced by the parser and transforms it into bytecode instructions that the VM can execute.

## Architecture

The compiler is built on `GenericCompiler` from the `bytecode_compiler` package. It registers handler functions for each of the ~55 Starlark grammar rules:

- **Opcodes**: 46 Starlark-specific bytecode instruction codes
- **Rule Handlers**: ~55 functions that handle each grammar rule
- **Operator Maps**: Mappings from operator symbols to opcodes

## How It Fits in the Stack

```
Source Code -> Lexer -> Parser -> **Compiler** -> Virtual Machine
```

## Usage

```elixir
alias CodingAdventures.StarlarkAstToBytecodeCompiler

# Compile source code to bytecode
code_object = StarlarkAstToBytecodeCompiler.compile_starlark("x = 1 + 2\n")

# Or compile from an AST
code_object = StarlarkAstToBytecodeCompiler.compile_ast(ast)
```

## Dependencies

- `coding_adventures_virtual_machine` — CodeObject, Instruction types
- `coding_adventures_bytecode_compiler` — GenericCompiler framework
