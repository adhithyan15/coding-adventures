# Specifications

This folder contains the specifications for every package in the coding-adventures monorepo. Specs are the blueprint — they describe **what** a package does, **why** it exists, **how** it connects to other layers, and **what the public API** looks like.

## Rules

1. Specs are written and committed **before** any implementation code
2. Specs are living documents — update them as understanding deepens
3. Every package has exactly one spec document

## Numbering

Specs are numbered by their layer in the computing stack, bottom to top:

| # | Spec | Layer |
|---|------|-------|
| 01 | Logic Gates | Hardware |
| 02 | Arithmetic | Hardware |
| 03 | CPU Simulator | Hardware |
| 04 | ARM Simulator | Hardware/ISA |
| 05 | Assembler | Software (low-level) |
| 06 | Lexer | Software (compiler) |
| 07 | Parser | Software (compiler) |
| 08 | Bytecode Compiler | Software (compiler) |
| 09 | Virtual Machine | Software (runtime) |

## Spec Template

Each spec follows this structure:

```markdown
# Package Name

## Overview
What this package does and why it exists.

## Layer Position
Where it sits in the stack and what it connects to.

## Concepts
Key concepts the reader needs to understand.

## Public API
What functions/classes the package exposes.

## Data Flow
What comes in, what goes out.

## Test Strategy
How correctness is verified.

## Future Extensions
What could be added later.
```
