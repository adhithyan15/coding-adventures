# virtual-machine

A generic, pluggable bytecode execution engine in Rust.

## What is this?

This crate provides the execution machinery for a stack-based virtual machine — the component that actually runs bytecode. Instead of being hardcoded to one language (like the JVM is to Java), it lets you plug in your own opcodes and handlers. Think of it like a CPU where you define the instruction set.

## Where does it fit in the stack?

```
Source Code → Lexer → Parser → AST → Compiler → CodeObject → **Virtual Machine** → Output
```

The virtual machine is the final stage: it takes compiled bytecode (a `CodeObject`) and executes it, producing output.

## Key Types

- **`GenericVM`** — The pluggable VM. Register opcode handlers, then call `execute()`.
- **`CodeObject`** — A compiled unit of code (instructions + constants + names).
- **`Value`** — Stack values: Int, Float, Str, Bool, Code, Null.
- **`Instruction`** — An opcode + optional operand.
- **`VMTrace`** — A snapshot of VM state at each execution step.
- **`VMError`** — Typed errors (no panics).

## Usage

```rust
use virtual_machine::*;

// 1. Create a VM
let mut vm = GenericVM::new();

// 2. Register handlers for your opcodes
vm.register_opcode(opcodes::LOAD_CONST, my_load_const_handler);
vm.register_opcode(opcodes::ADD, my_add_handler);
vm.register_opcode(opcodes::HALT, my_halt_handler);

// 3. Create a CodeObject (usually from a compiler)
let code = CodeObject {
    instructions: vec![/* ... */],
    constants: vec![Value::Int(10), Value::Int(20)],
    names: vec![],
};

// 4. Execute!
let traces = vm.execute(&code).unwrap();
```

## Standard Opcodes

The `opcodes` module defines standard opcodes shared by many languages:

| Category   | Opcodes                                        |
|------------|------------------------------------------------|
| Stack      | LOAD_CONST, POP, DUP                           |
| Variables  | STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL  |
| Arithmetic | ADD, SUBTRACT, MULTIPLY, DIVIDE, MODULO        |
| Comparison | EQUAL, NOT_EQUAL, LESS_THAN, etc.               |
| Logic      | AND, OR, NOT                                    |
| Control    | JUMP, JUMP_IF_TRUE, JUMP_IF_FALSE               |
| Functions  | CALL, RETURN                                    |
| I/O        | PRINT                                           |
| System     | HALT                                            |
