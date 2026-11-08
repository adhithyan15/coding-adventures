# Changelog

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
