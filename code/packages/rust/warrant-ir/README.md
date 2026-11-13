# warrant-ir

Warrant Intermediate Representation (WIR) and core interpreter for [VER25](../../../specs/VER25-wir-core-interpreter.md).

Defines the WIR core abstract syntax tree, static well-formedness and type checking, step cost accounting, observable trace generation, memory safety evaluation with explicit `Stuck` states, and constant-time verification.

## Features

- **Core AST**: Strongly typed expressions, statements, and basic-block control flow graph.
- **Static Checking**: CFG validation, single-assignment within blocks, secrecy propagation rules.
- **Trace Observation**: Branch conditions, memory access patterns, and capability invocations.
- **Constant-Time Verification**: Dual-execution check ensuring secret inputs produce identical observable traces.
- **Memory Model**: Provenance-tracked allocations preventing use-after-free and out-of-bounds accesses.

## Proof obligations from Warrant (VER04h)

`inline` splices callee bodies into a function so provers analyse a public
export itself. `tests/warrant_proofs.rs` reads `ct-compare.warrant`, lowers the
real source and proves 2-safety / step bounds for every length shape up to a
stated bound, writing content-addressed DRAT files and a manifest when
`WARRANT_PROOF_OUT=<dir>` is set. Labels are `bounded(K)`, never `proved`, for
byte-slice operations.

`tests/gf256_proofs.rs`, `tests/wasm_leb128_proofs.rs`,
`tests/logic_gates_proofs.rs`, `tests/lzw_proofs.rs` and `tests/arithmetic_proofs.rs` do the same for
`gf256.warrant`, `wasm-leb128.warrant`, `logic-gates.warrant`, `lzw.warrant` and
`arithmetic.warrant` (logic-gates and arithmetic read two source files each:
`gates.rs` and `combinational.rs`, `adders.rs` and `alu.rs`). The front end refuses all the real
sources, so they record every constant-time and steps-bound obligation
`not-live` with the front end's own message (and heap/depth rows `unchecked`)
rather than a proof; each has a `lowering-blocker` test that fails the day the
source becomes lowerable.
