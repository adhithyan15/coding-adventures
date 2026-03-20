# bytecode-compiler

A generic, pluggable AST-to-bytecode compiler in Rust.

## What is this?

This crate provides a framework for compiling Abstract Syntax Trees (ASTs) into bytecode that the `virtual-machine` crate can execute. Like the VM, it's pluggable: you register handlers for different AST node types, and the compiler dispatches to them as it walks the tree.

## Where does it fit in the stack?

```
Source Code → Lexer → Parser → AST → **Compiler** → CodeObject → Virtual Machine → Output
```

The compiler sits between the parser and the VM. It takes the tree-structured AST and flattens it into a linear sequence of bytecode instructions.

## Key Types

- **`GenericCompiler`** — The pluggable compiler. Register rule handlers, then call `compile()`.
- **`ASTNode`** — A node in the AST, with a rule name and children.
- **`ASTChild`** — Either a sub-node or a terminal token.
- **`TokenNode`** — A terminal token (type + value).
- **`CompilerScope`** — Compilation state for a single code unit (function body, etc.).

## Usage

```rust
use bytecode_compiler::*;
use virtual_machine::*;

// 1. Create a compiler
let mut compiler = GenericCompiler::new();

// 2. Register handlers for AST rule names
compiler.register_rule("number", compile_number);
compiler.register_rule("add", compile_add);
compiler.register_rule("print", compile_print);

// 3. Compile an AST
let code = compiler.compile(&ast, Some(opcodes::HALT));

// 4. Execute with a VM
let mut vm = GenericVM::new();
// ... register VM handlers ...
vm.execute(&code).unwrap();
```

## Features

- **Jump patching**: `emit_jump()` + `patch_jump()` for control flow (if/else, loops).
- **Scoping**: `enter_scope()` / `exit_scope()` for compiling functions.
- **Nested compilation**: `compile_nested()` for compiling function bodies in isolation.
- **Name deduplication**: `add_name()` reuses existing entries.
