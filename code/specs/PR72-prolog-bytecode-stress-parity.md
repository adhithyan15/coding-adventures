# PR72: Prolog Bytecode VM Stress Parity

## Goal

Prove that the new Prolog -> `logic-bytecode` -> `logic-bytecode-vm` execution
path can carry realistic Prolog workloads, not just simple facts and recursive
toy queries.

PR71 made the bytecode VM path executable. This follow-up gives it a broad
regression harness against the mature structured `logic-vm` path.

## Scope

- Add bytecode VM stress tests that compare named answers against the existing
  structured VM helpers.
- Cover recursive search, linked modules, DCG expansion, arithmetic,
  collections, dynamic initialization, exception handling, cleanup control,
  term metadata, text conversion, flags, list stdlib predicates, higher-order
  predicates, and CLP(FD) modeling globals.
- Keep the tests at the Prolog source level so the whole frontend stack is
  exercised before bytecode lowering.

## Acceptance

- Every bytecode stress case must return the same named answers as the
  structured VM path.
- Dynamic initialization must seed state before bytecode-backed source queries.
- CLP(FD), module, DCG, aggregation, and cleanup/control features must all
  survive bytecode compilation and execution.

## Still Out Of Scope

These tests validate semantic parity for the loader-bytecode VM. They do not
add WAM-style proof-search bytecode, indexing, or performance assertions.
