# warrant-ir

Warrant Intermediate Representation (WIR) and core interpreter for [VER25](../../../specs/VER25-wir-core-interpreter.md).

Defines the WIR core abstract syntax tree, static well-formedness and type checking, step cost accounting, observable trace generation, memory safety evaluation with explicit `Stuck` states, and constant-time verification.

## Features

- **Core AST**: Strongly typed expressions, statements, and basic-block control flow graph.
- **Static Checking**: CFG validation, single-assignment within blocks, secrecy propagation rules.
- **Trace Observation**: Branch conditions, memory access patterns, and capability invocations.
- **Constant-Time Verification**: Dual-execution check ensuring secret inputs produce identical observable traces.
- **Memory Model**: Provenance-tracked allocations preventing use-after-free and out-of-bounds accesses.
