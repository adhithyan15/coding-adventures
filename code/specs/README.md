# Specifications

This folder contains the specifications for every package in the coding-adventures monorepo. Specs are the blueprint — they describe **what** a package does, **why** it exists, **how** it connects to other layers, and **what the public API** looks like.

## Rules

1. Specs are written and committed **before** any implementation code
2. Specs are living documents — update them as understanding deepens
3. Every package has exactly one spec document

## Numbering

Specs are numbered **top-down** — from the user-facing language down to silicon. This matches the order you experience computing: you write code, it gets compiled, and eventually runs on hardware.

| # | Spec | Layer |
|---|------|-------|
| 01 | High-level language | Source code (not a package — this is what you write) |
| 02 | Lexer | Compiler frontend |
| 03 | Parser | Compiler frontend |
| 04 | Bytecode Compiler | Compiler backend |
| 05 | Virtual Machine | Runtime (interpreted path) |
| 06 | Assembler | Compiler backend (compiled path) |
| 07a | RISC-V Simulator | Instruction Set Architecture |
| 07b | ARM Simulator | Instruction Set Architecture |
| 07c | WASM Simulator | Instruction Set Architecture |
| 07d | Intel 4004 Simulator | Instruction Set Architecture |
| 07e | ARM1 Simulator | Instruction Set Architecture |
| 07f | Intel 8008 Simulator | Instruction Set Architecture |
| 07g | GE-225 Simulator | Instruction Set Architecture |
| 08 | CPU Simulator | Hardware |
| 09 | Arithmetic (ALU) | Hardware |
| 10 | Logic Gates | Hardware |
| 11 | Transistors | Hardware (foundation) |

**Tooling specs** (not numbered by layer):
| # | Spec | Purpose |
|---|------|---------|
| 10-pipeline | Pipeline Orchestrator | Chains all layers together |
| 11-html-visualizer | HTML Visualizer | Renders pipeline output as HTML |

**Interactive hardware specs**:
| # | Spec | Purpose |
|---|------|---------|
| BVM00 | Board VM Architecture | Interactive physical-board runtime and package map |
| BVM01 | Board VM Binary Protocol | Compact transport-agnostic host/board protocol |
| BVM02 | Board VM Bytecode IR | Portable bytecode and capability calls |
| BVM03 | Board VM Rust Runtime | `no_std` firmware runtime and board HAL contract |
| BVM04 | Board VM Host SDKs | Language-agnostic SDK and REPL contract |
| BVM05 | Board VM Blink MVP | First end-to-end Arduino-style blink scenario |

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
