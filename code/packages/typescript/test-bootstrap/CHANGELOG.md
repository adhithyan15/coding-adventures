# Changelog

## Unreleased

- Fixed the canonical `ct-compare.warrant` tests, which asserted 1 claim and 13 results; the spec now has 9 claims (VER-P030), so they expect 9 claims and 21 results (12 examples + 9 claims).
- Raised the claim-refinement rejection-sampling cap in `src/warrant.ts` from 100 to 1000 attempts: `len(a) == len(b)` over two independent `Bytes` binders is accepted ~1 time in 33, so `select_picks` failed deterministically with "Generator filter exhausted". Mirrors the same fix in `warrant-compiler`.

## 0.5.0 (2026-11-08)

- Level 3 CDCL SAT Solver & Tseitin Bit-Blaster (VER09 / VER09a).
- Conflict-Driven Clause Learning (CDCL) solver core with integer variables and literals (`Var`, `Lit`, MiniSat convention).
- Two-watched-literals (2WL) unit propagation engine with blocker literal caching.
- 1-UIP conflict analysis, learned clause recording, and non-chronological backjumping.
- VSIDS variable decision heuristic with activity decays and phase saving.
- Luby restart schedule and learned-clause database reduction.
- Incremental solving under assumptions (`solveWithAssumptions`) and failed-assumption core extraction.
- Hostile input protection: conflict budget (`maxConflicts`) and propagation step budget (`maxSteps`).
- Tseitin transformation and bit-blaster (`CnfBuilder`): logical gates (`AND`, `OR`, `XOR`, `ITE`, `NOT`, `EQUIV`), 1-bit full adders, and multi-bit ripple-carry adders.
- DIMACS CNF reader (`parseDimacs`) and writer (`writeDimacs`).
- Zero third-party npm runtime dependencies (`"dependencies": {}`).

- Level 1 Warrant Compiler & Claim Verifier (VER04g).
- Bounded lexer & layout analyzer: comment stripping, indentation and layout capture (`INDENT`/`DEDENT`), hex byte literals `b"..."`, number suffixes.
- Recursive-descent guarded parser: modules, nominal error types, mathematical reference models, operation contracts, input contracts (`hostile`, `rejects ... with ...`, `ensures`), and resource bounds.
- Pure ground model evaluator: bounded call depth (<= 64) and work steps.
- Test suite compilation: executes ground `example`s as Level 1 unit tests and universal `claim`s (`forall`) as Level 2 property-based tests via `assertProperty` and `Gen`.
- Full execution and validation of canonical `code/specs/ct-compare.warrant` (11 examples, 1 claim, input rejection, freshness checks).

## 0.3.0 (2026-11-08)

- Level 2 Coverage-Guided Fuzzing Engine (VER06 / VER06b).
- Closed mutator algebra: `bitFlip`, `byteFlip`, `byteArithmetic`, `boundaryInsert`, `blockDelete`, `blockInsert`, `blockDuplicate`, `splice`, and compound `mutateSequence`.
- 65,536-bit (8,192-byte) coverage bitmap interface (`CoverageBitmap`, `corpusAdmit`).
- Power schedule energy allocation favoring short inputs with high novel edge discovery.
- Crash deduplication with normalized FNV-1a 64-bit signature hashing (`crashSignature`, `isDuplicateCrash`).
- Crashing input minimization (`minimizeCrashingInput`) via recursive block deletion and byte zeroing.
- Closed failure vocabulary enforcement (`fuzz-crash`, `fuzz-timeout`, `fuzz-oom`, `fuzz-leak`, `fuzz-panic`, `isFuzzFailureId`).
- Privacy-preserving TAP reporting (`formatTapFailure`) strictly redacting raw payload bytes.
- Full validation of canonical `fuzz-v1` conformance corpus (25 cases in `cases.tfv` / `cases.json`).

## 0.2.0 (2026-11-08)

- Level 2 Property-Based Testing (PBT) engine (VER05 / VER05a).
- Generator combinator algebra (`Gen<T>`): primitive generators (`bool`, `u8`, `u64`, `integer`, `bytes`), combinators (`map`, `filter`, `list`, `oneOf`).
- Integrated shrinking protocol (`shrinkInt`, `shrinkList`, `shrinkBytes`) preserving type refinements and invariant boundaries.
- Deterministic SplitMix64 pseudo-random generation with isolated sub-seed derivation.
- Full validation of canonical `pbt-v1` conformance corpus (25 cases in `cases.tfv` / `cases.json`).
- Closed failure vocabulary reporting (`property-counterexample`, `property-exhausted`, `property-timeout`, `property-panic`).
- High-level property assertion API (`assertProperty`, `checkProperty`).

## 0.1.0 (2026-11-08)

- Initial release of `@coding-adventures/test-bootstrap`.
- Strict TFV v1 parser and validator (TF01 §2).
- Pure deep equality algorithm matching TF01 §3.1 (int/bigint, canonical f64 NaN, -0 !== +0, bytes, text, list, map).
- Deterministic SplitMix64 pseudo-random generator, FNV-1a 64 test-id hashing, Fisher-Yates array shuffling, and isolated sub-seeds (TF01 §5).
- TAP version 14 test reporter and failure-detecting sabotage mode (TF01 §6).
- Ergonomic Level 1 matchers (`describe`, `it`, `test`, `expect`, `beforeEach`, `afterEach`) over Node 22 native `node:test`.
- Declarative CLI test runner powered by internal `@coding-adventures/cli-builder` (`testBootstrapCliSpec`, TF01b).
- Multi-target execution: self-corpus runner, native test file execution, `--seed`, `--sabotage`, `--filter`, `--reporter`, and fail-closed exit codes.
- Zero external runtime dependencies.
