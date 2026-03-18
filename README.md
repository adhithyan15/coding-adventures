# Coding Adventures

A polyglot learning monorepo for building the entire computing stack from scratch — from logic gates to high-level programs.

## Vision

Computers are deterministic. There is no magic. This repo exists to prove that by building every layer of the computing stack by hand:

```
Layer 1: Logic Gates         AND, OR, NOT, XOR — the foundation of all digital logic
Layer 2: Arithmetic          Half adders, full adders, ALU — how numbers are computed
Layer 3: CPU Simulator       Registers, memory, fetch-decode-execute cycle
Layer 4: ARM Simulator       Real instruction set, decode and execute ARM machine code
Layer 5: Assembler           Translate human-readable assembly to binary machine code
Layer 6: Lexer               Break source code into tokens
Layer 7: Parser              Build abstract syntax trees from tokens
Layer 8: Bytecode Compiler   Turn ASTs into virtual machine instructions
Layer 9: Virtual Machine     Execute bytecode — the heart of Python, Ruby, Java
```

The end goal: write `x = 1 + 2` in Python and watch it propagate through every layer — from source code down to logic gates.

## Structure

```
code/
  specs/       — Specifications for each package (the blueprint)
  learning/    — Notes and learning materials per language/topic
  packages/    — Publishable libraries (organized by language)
  programs/    — Standalone programs (organized by language)
```

## Languages

Starting with Python, then Ruby and TypeScript. Each package will eventually be implemented in multiple languages to deepen understanding of each ecosystem.

## Philosophy

- Go slow and deliberate
- Build from scratch — no existing lexers, parsers, or frameworks
- Understand every layer before moving to the next
- Every package is publishable, tested (>80% coverage), and documented
- Specs first, then tests, then implementation

## Copyright

Everything in this repo is copyrighted to Adhithya Rajasekaran. Individual packages may be licensed separately.
