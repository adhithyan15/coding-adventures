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
| KILN00 | Kiln Build System | Language-agnostic build system: hashed keys, isolated work roots, CI pass ledger; supersedes B01–B04 |
| KILN02 | Kiln Key Encoding | Byte-exact, length-framed key preimage with golden vectors |
| KILN03 | Kiln Legacy Keys | Keys for today's shell BUILDs from the old tool's graph export |
| KILN04 | Kiln Traced Read-Sets | Reuse legacy passes only when every traced read would see the same thing again |

**Interactive hardware specs**:
| # | Spec | Purpose |
|---|------|---------|
| BVM00 | Board VM Architecture | Interactive physical-board runtime and package map |
| BVM01 | Board VM Binary Protocol | Compact transport-agnostic host/board protocol |
| BVM02 | Board VM Bytecode IR | Portable bytecode and capability calls |
| BVM03 | Board VM Rust Runtime | `no_std` firmware runtime and board HAL contract |
| BVM04 | Board VM Host SDKs | Language-agnostic SDK and REPL contract |
| BVM05 | Board VM Blink MVP | First end-to-end Arduino-style blink scenario |
| BVM06 | Board VM Target Matrix | Explicit board/ISA/backend support plan |

## Verified-shipping pilot

| Spec | Contract / current boundary |
|---|---|
| [VER00](VER00-verified-shipping-program.md) | Parent verification program; publishing/approval blockers remain in §12.1 |
| [VER26](VER26-ct-compare-pilot.md) | Rust ct-compare pilot and evidence requirements |
| [TF01](TF01-bootstrap-harness.md) | Implemented Rust L0 harness, TFV transport and closed failure vocabulary |
| [CT01](CT01-constant-time-compare.md) | Portable operations and corpus; logical work is not a timing proof |
| [LEB01](LEB01-wasm-leb128.md) | LEB128 contract (canonical encoders, malformed-input rules, bytes-consumed bound) and its Warrant document [`wasm-leb128.warrant`](wasm-leb128.warrant) |
| [VER04](VER04-warrant-language.md) | Warrant v0 specification and [grammar artifacts](../packages/rust/warrant-grammar/README.md); guarded source frontend, checker and generation follow separately |
| [Warrant design inventory](fixtures/warrant-v0/README.md) | Named future acceptance cases; not an executed conformance corpus |
| [VER27](VER27-layered-validation-roadmap.md) | Layered validation roadmap: vision, layer ladder, per-language assumption tiers, thread order |
| [VER29](VER29-layer-contract-linking.md) | Layer contract linking (LV-005): `contract.<lane>.<package>.<claim>` ids, the generated `contract` family, fail-closed discharge, strength propagation, coverage and reach-around rules, CI re-selection, implementation slices; amendments to VER04 and VER13 as text |
| [Delivery backlog](VER-BACKLOG.md) | Current bounded work, prerequisites and deferred ownership |

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
