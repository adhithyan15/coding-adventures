# Hello World

The first program — the starting point for the entire coding-adventures project.

## What it does

Prints `Hello, World!` to the console. That's it — but the long-term goal is to trace this simple program all the way down through every layer of the computing stack:

```
Source code (this file)
→ Lexer (tokenize)
→ Parser (build AST)
→ Compiler (emit bytecode or ARM assembly)
→ Virtual Machine (execute bytecode)
→ ARM Simulator (execute machine instructions)
→ CPU Simulator (fetch-decode-execute cycle)
→ ALU (arithmetic operations)
→ Logic Gates (AND, OR, NOT — the foundation)
```

## Usage

```bash
ruby hello_world.rb
```

## How it fits in the stack

This is the input program that will eventually be fed through every package in the computing stack, from the lexer down to logic gates. It is the "end-to-end test" for the entire project.
