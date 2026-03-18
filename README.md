# Coding Adventures

A polyglot learning monorepo for building the entire computing stack from scratch — from logic gates to high-level programs.

## Vision

Computers are deterministic. There is no magic. This repo exists to prove that by building every layer of the computing stack by hand:

```
 Layer 1:  Source Code            You write: x = 1 + 2
 Layer 2:  Lexer                  Breaks source text into tokens
 Layer 3:  Parser                 Builds abstract syntax trees from tokens
 Layer 4:  Bytecode Compiler      Turns ASTs into VM instructions (or assembly)
                │
                ├─── Interpreted path ─────────────────────────┐
                │                                              │
 Layer 5:  Virtual Machine        Executes bytecode (like CPython, JVM)
                │                                              │
                ├─── Compiled path ────────────────────────────┘
                │
 Layer 6:  Assembler              Translates assembly → binary machine code
 Layer 7:  Instruction Set        RISC-V, ARM, WASM, Intel 4004
                │                 Decodes binary → CPU operations
 Layer 8:  CPU Simulator          Fetch-decode-execute cycle, registers, memory
 Layer 9:  Arithmetic (ALU)       Addition, subtraction — built from logic gates
 Layer 10: Logic Gates            AND, OR, XOR — the irreducible foundation
```

Each layer is a Russian nesting doll — the ISA simulators use the CPU, which uses the ALU, which uses logic gates. Every layer delegates downward.

Plus orchestration and visualization:
- **Pipeline** — chains all layers into a single execution flow, exports JSON
- **HTML Renderer** — generates beautiful static HTML reports from pipeline JSON
- **Pipeline Visualizer** — runs the pipeline and produces a self-contained HTML file

The end goal: write `x = 1 + 2` in Python, run the pipeline, and open a single HTML file that shows every stage — from source code down to logic gates. Any language implementation (Python, Ruby, TypeScript) can produce the same visualization.

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
