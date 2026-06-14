# Symbolic VM / CAS / MACSYMA TypeScript and Rust Port Roadmap

> Status: implementation roadmap for the multi-language symbolic stack.
> Parent specs: `symbolic-computation.md`, `macsyma-runtime.md`,
> `macsyma-repl.md`, `cas-*.md`, and `phase*.md` CAS specs.

## Goal

Port the symbolic VM, CAS substrates, and MACSYMA implementation so the same
symbolic programs can run through two first-party execution paths:

1. A pure TypeScript implementation that runs in Node.js and browsers with no
   host APIs, native extensions, or server dependency.
2. A Rust implementation that can be compiled to WebAssembly for users that
   want the Rust CAS engine in the browser.

The TypeScript path is not a thin WASM wrapper. It is an independent pure-JS
runtime. The Rust path exists in parallel as a WASM-capable engine and should
eventually reach the same semantic coverage as the Python reference.

## Current Inventory

### Python Reference Stack

The Python implementation is the semantic source of truth today:

- Core: `symbolic-ir`, `symbolic-vm`
- MACSYMA: `macsyma-lexer`, `macsyma-parser`, `macsyma-compiler`,
  `macsyma-runtime`
- CAS substrates: `cas-simplify`, `cas-substitution`, `cas-pattern-matching`,
  `cas-pretty-printer`, `cas-factor`, `cas-solve`, `cas-list-operations`,
  `cas-matrix`, `cas-limit-series`, `cas-number-theory`, `cas-complex`,
  `cas-trig`, `cas-algebraic`, `cas-fourier`, `cas-laplace`, `cas-mnewton`,
  `cas-multivariate`, `cas-ode`, `cas-ode-numeric`, `cas-summation`

### Rust Coverage

Rust already has a partial generic CAS stack:

- Present: `symbolic-ir`, `symbolic-vm`, `cas-simplify`,
  `cas-substitution`, `cas-pattern-matching`, `cas-pretty-printer`,
  `cas-factor`, `cas-solve`, `cas-list-operations`, `cas-matrix`,
  `cas-limit-series`, `cas-number-theory`, `cas-complex`, `cas-trig`
- Missing versus Python: `macsyma-lexer`, `macsyma-parser`,
  `macsyma-compiler`, `macsyma-runtime`, `cas-algebraic`, `cas-fourier`,
  `cas-laplace`, `cas-mnewton`, `cas-multivariate`, `cas-ode`,
  `cas-ode-numeric`, `cas-summation`

### TypeScript Coverage

TypeScript has no package-level symbolic/CAS/MACSYMA stack yet. There are
general TypeScript grammar/runtime packages in the repo, but no
`symbolic-ir`, `symbolic-vm`, `cas-*`, or `macsyma-*` TypeScript packages.

## Architecture

```text
MACSYMA source
  |
  v
macsyma-lexer      macsyma-parser      macsyma-compiler
  |                    |                    |
  +--------------------+--------------------+
                                           |
                                           v
                                      symbolic-ir
                                           |
                                           v
                                      symbolic-vm
                                           |
          +--------------------------------+--------------------------------+
          |                                |                                |
     CAS handlers                     MACSYMA runtime                 Pretty output
          |
          v
  simplify / solve / factor / matrix / limit / trig / ...
```

Both TypeScript and Rust should keep this layer split. `symbolic-ir` has no
dependencies. `symbolic-vm` depends on `symbolic-ir`. CAS substrates depend on
the core layers, not on MACSYMA. MACSYMA packages depend downward into the core
and CAS packages.

## Phase Plan

### Phase 1: Core Pure-JS Foundation

Deliver TypeScript `symbolic-ir` and `symbolic-vm` packages.

Acceptance criteria:

- Runs under Node.js and browser bundlers with no `fs`, `path`, `process`,
  native addon, or DOM dependency.
- Defines the six IR node forms: symbol, integer, rational, float, string,
  and apply.
- Preserves exact integer/rational arithmetic with `bigint`.
- Exposes structural equality, display, and stable structural keys for maps.
- Implements the generic VM evaluator with held heads, backends, handlers,
  assignments, simple function definitions, and user-function application.
- Ships strict and symbolic backends.
- Handles arithmetic, elementary numeric functions, comparisons, boolean
  logic, `If`, `Assign`, `Define`, and `List`.
- Has focused tests for exact rational arithmetic, symbolic identity folding,
  binding, function calls, strict failures, and unknown-head passthrough.

This phase unblocks all later TypeScript CAS work and gives browser code a real
pure-JS symbolic substrate.

### Phase 2: MACSYMA Frontend in TypeScript

Deliver TypeScript `macsyma-lexer`, `macsyma-parser`, and
`macsyma-compiler`.

Acceptance criteria:

- Tokenizes and parses the grammar in `code/grammars/macsyma`.
- Compiles statements to the same canonical IR shapes as Python.
- Supports arithmetic precedence, lists, assignment, function definitions,
  function calls, comparison, logic, strings, and statement terminators.
- Includes golden parity tests against Python-produced IR strings or JSON
  fixtures for representative programs.

This phase can run in parallel with Rust MACSYMA frontend work because both
consume the same grammar/spec and target the same `symbolic-ir` shape.

### Phase 3: Generic TypeScript CAS Substrates

Port language-neutral CAS packages in dependency order:

1. `cas-pretty-printer`
2. `cas-substitution`
3. `cas-simplify`
4. `cas-list-operations`
5. `cas-pattern-matching`
6. `cas-factor`
7. `cas-solve`
8. `cas-matrix`
9. `cas-limit-series`
10. `cas-number-theory`
11. `cas-complex`
12. `cas-trig`

Acceptance criteria:

- Each package is browser-safe TypeScript.
- Each package includes Python/Rust parity fixtures for its public handlers.
- `symbolic-vm` can register the new handlers without MACSYMA dependencies.

### Phase 4: Advanced TypeScript CAS Phases

Port the later Python-only CAS packages:

- `cas-algebraic`
- `cas-fourier`
- `cas-laplace`
- `cas-mnewton`
- `cas-multivariate`
- `cas-ode`
- `cas-ode-numeric`
- `cas-summation`

These packages are algorithmically deeper and should be ported after the core
handler registration story is stable.

### Phase 5: TypeScript MACSYMA Runtime and REPL Surface

Deliver `macsyma-runtime` on top of the TypeScript symbolic VM:

- History: `%`, `%iN`, `%oN`
- `kill`, `ev`, `block`, `assume`, `forget`, `is`
- MACSYMA option flags and name table
- Display/suppress statement wrappers
- Browser-friendly REPL/session API

This phase should not import Node APIs; browser shells can provide their own UI.

### Phase 6: Rust Completion for WASM

Fill the Rust gaps versus Python:

1. MACSYMA frontend/runtime:
   `macsyma-lexer`, `macsyma-parser`, `macsyma-compiler`,
   `macsyma-runtime`
2. Advanced CAS:
   `cas-algebraic`, `cas-fourier`, `cas-laplace`, `cas-mnewton`,
   `cas-multivariate`, `cas-ode`, `cas-ode-numeric`, `cas-summation`
3. WASM packaging:
   a small `symbolic-wasm` facade that exposes parse/evaluate/format calls
   with stable JSON-serializable boundaries.

Rust crates must avoid non-WASM-compatible dependencies on the hot path. Any
optional host integration should be behind features that are disabled for
`wasm32-unknown-unknown`.

## Parallel Work Streams

The work can proceed in parallel without waiting for every PR to merge:

- Stream A: TypeScript core and MACSYMA frontend
- Stream B: TypeScript CAS substrates in dependency order
- Stream C: Rust MACSYMA frontend/runtime
- Stream D: Rust advanced CAS package completion
- Stream E: cross-language parity fixtures and WASM/browser harnesses

The first PR in each stream should be intentionally narrow. After a stream has
one merged foundation PR, later PRs can stack package-by-package.

## First PR Scope

This PR implements Phase 1 only:

- Add `code/packages/typescript/symbolic-ir`
- Add `code/packages/typescript/symbolic-vm`
- Add focused tests
- Document the overall roadmap in this file

It does not port MACSYMA syntax or advanced CAS algorithms yet. That boundary is
intentional: every later TypeScript package needs a stable, reviewed IR/VM base.

## Validation

For this PR:

```bash
cd code/packages/typescript/symbolic-ir && npm install && npm test && npm run build
cd ../symbolic-vm && npm install && npm test && npm run build
```

For later Rust PRs:

```bash
cd code/packages/rust
cargo test -p symbolic-ir -p symbolic-vm
cargo test -p <new-rust-cas-package>
```

Before finishing Rust work, remove `code/packages/rust/Cargo.lock` if Cargo
creates it, matching repo policy.
