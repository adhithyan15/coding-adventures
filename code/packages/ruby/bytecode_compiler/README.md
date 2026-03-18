# coding_adventures_bytecode_compiler

Bytecode compiler with four backends: our custom VM, JVM, CLR, and WebAssembly.

## What It Does

This gem compiles Abstract Syntax Trees (from the parser) into bytecode for
execution by virtual machines. It is the bridge between parsing and execution --
Layer 4a of the coding-adventures computing stack.

Four compiler backends compile the same AST to different targets:

- **Compiler** -- Targets our custom stack-based VM (CodeObject output)
- **JVMCompiler** -- Targets the Java Virtual Machine (real JVM bytecode bytes)
- **CLRCompiler** -- Targets the .NET Common Language Runtime (real CLR IL bytes)
- **WASMCompiler** -- Targets WebAssembly (real WASM bytecode bytes)

## How It Fits in the Stack

```
Source code -> Lexer -> Parser -> [Bytecode Compiler] -> Virtual Machine
                                       |
                                  4 backends:
                                  - Custom VM
                                  - JVM
                                  - CLR
                                  - WASM
```

## Usage

```ruby
require "coding_adventures_bytecode_compiler"

# End-to-end: source code -> CodeObject
code = CodingAdventures::BytecodeCompiler.compile_source("x = 1 + 2")

# Step by step: AST -> CodeObject
compiler = CodingAdventures::BytecodeCompiler::Compiler.new
code = compiler.compile(ast)

# JVM backend
jvm_code = CodingAdventures::BytecodeCompiler::JVMCompiler.new.compile(ast)

# CLR backend
clr_code = CodingAdventures::BytecodeCompiler::CLRCompiler.new.compile(ast)

# WASM backend
wasm_code = CodingAdventures::BytecodeCompiler::WASMCompiler.new.compile(ast)
```

## Dependencies

- `coding_adventures_lexer` -- Tokenizes source code
- `coding_adventures_parser` -- Parses tokens into AST
- `coding_adventures_virtual_machine` -- Provides CodeObject, Instruction, OpCode
