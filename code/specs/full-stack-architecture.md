# Full Stack Architecture

## Overview

This document describes how all packages in the computing stack connect to form a complete pipeline from high-level source code to logic gate operations.

## The Pipeline

```
"x = 1 + 2"                          Source code (text)
     │
     ▼
┌──────────┐
│  LEXER   │  Breaks text into tokens: NAME('x') EQUALS NUMBER(1) PLUS NUMBER(2)
└──────────┘
     │ tokens
     ▼
┌──────────┐
│  PARSER  │  Builds a tree: Assignment(target=x, value=BinaryOp(1, +, 2))
└──────────┘
     │ AST
     ▼
┌──────────────────┐
│ BYTECODE COMPILER│  Emits stack instructions: LOAD_CONST 1, LOAD_CONST 2, ADD, STORE x
└──────────────────┘
     │ bytecode
     ▼
┌─────────────────┐
│ VIRTUAL MACHINE │  Executes bytecode using a stack: push 1, push 2, pop+pop→add→push 3, store
└─────────────────┘
     │
     │  (Alternative paths: compiler emits assembly instead of bytecode)
     │
     ▼
┌──────────┐
│ ASSEMBLER│  Converts assembly → binary machine code
└──────────┘
     │ machine code bytes
     ▼
┌──────────────────┐   ┌────────────────┐
│ RISC-V SIMULATOR │   │ ARM SIMULATOR  │  (choose one target)
│ addi, add, ecall │   │ MOV, ADD, etc. │
└──────────────────┘   └────────────────┘
     │ operations (add, store, load)
     ▼
┌────────────────┐
│ CPU SIMULATOR  │  Fetch-decode-execute cycle, registers, memory, program counter
└────────────────┘
     │ ALU operations
     ▼
┌──────────────┐
│  ARITHMETIC  │  Full adder chains, ripple carry, ALU operations built from gates
└──────────────┘
     │ gate operations
     ▼
┌──────────────┐
│ LOGIC GATES  │  AND, OR, NOT, XOR — the irreducible foundation
└──────────────┘
```

## Two Execution Paths

The stack supports two paths for executing code:

### Path A: Interpreted (VM)
```
Source → Lexer → Parser → Bytecode Compiler → Virtual Machine
```
This is how CPython works. The VM interprets bytecode instruction by instruction.

### Path B: Compiled to RISC-V (Full stack)
```
Source → Lexer → Parser → RISC-V Compiler → Assembler → RISC-V Simulator → CPU → ALU → Gates
```
This traces execution all the way down to individual logic gates. RISC-V is the primary target due to its clean, regular encoding.

### Path C: Compiled to ARM
```
Source → Lexer → Parser → ARM Compiler → Assembler → ARM Simulator → CPU → ALU → Gates
```
Same as Path B but targeting ARMv7. Useful for comparing instruction set designs.

## Package Dependencies

```
logic-gates          ← no dependencies (foundation)
arithmetic           ← depends on logic-gates
cpu-simulator        ← depends on arithmetic
arm-simulator        ← depends on cpu-simulator
riscv-simulator      ← depends on cpu-simulator
assembler            ← depends on arm-simulator, riscv-simulator (for instruction encoding)
lexer                ← no dependencies (standalone text processing)
parser               ← depends on lexer (consumes tokens)
bytecode-compiler    ← depends on parser (consumes AST)
virtual-machine      ← depends on bytecode-compiler (executes bytecode)
pipeline             ← depends on ALL packages (orchestrator)
```

## Tools

### Pipeline Orchestrator
Chains all packages into a single `Pipeline.run("x = 1 + 2", target="vm")` call.
Captures stage snapshots for inspection and visualization.

### Stack Visualizer (TUI)
Terminal UI (Textual) that visually walks through every stage with step-through debugging.

## Shared Concepts

These concepts appear across multiple packages:

- **Binary representation**: Gates, arithmetic, assembler, ARM simulator all work with bits
- **Tree structures**: Parser produces trees, compiler walks trees
- **Stack operations**: ALU uses operand stacks, VM uses an execution stack
- **Instruction dispatch**: ARM simulator and VM both have fetch-decode-execute loops
- **Symbol tables**: Assembler, compiler, and VM all maintain name-to-value mappings

## Build Order

Packages should be built bottom-up (gates first) so each layer can use the one below it. However, the lexer/parser/compiler chain can be built in parallel since it starts independently and only connects to the hardware stack at the assembler level.
