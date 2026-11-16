# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Warrant adoption of the eight byte-string hashes (WX-P1a): the Warrant document
  `code/specs/hash-functions.warrant`, the contract
  `code/specs/DT17a-hash-functions-warrant.md`, `evidence.bindings`, the generated
  harness `tests/generated/warrant_evidence.rs` (273 tests, all passing: 67
  examples, 58 each of exhaustive, property and fuzz suites, 32 negative controls)
  and its manifest `evidence/test-manifest.json`, the shim
  `tests/warrant_evidence.rs`, and `evidence/proof-manifest.json` (written by
  `warrant-ir/tests/hash_functions_proofs.rs`; the `siphash_2_4` constant-time
  obligation is recorded `not-live`). This is the first harness that compiles the
  WX-FAULT1 fixed-width-integer arm of `flip-result-bit`.
- `tests/runtime_capability.rs` (VER08b): the eight covered hashes run on published
  and independently computed known answers inside a seccomp/Landlock jail derived
  from `required_capabilities.json` (no capabilities), watched by `strace`.
- `BUILD`, `BUILD_linux` (strict jail) and `BUILD_windows` run the unit and doc
  tests, the jail, `gen_evidence --check`, the evidence run and eight sabotage
  lines (seven `flip-result-bit`, one `missing-error`), then the proof manifest.
- Dev-dependencies `coding_adventures_test_bootstrap`,
  `coding_adventures_sha256` and `capability-runtime-conformance` for the above.

### Not wired

- The capability analyzer line and the `ca-assurance-report` package report
  (`assurance_report.json`, `effect_certificate.json`, `assurance-profile.json`)
  are not committed: see DT17a section 7. `analysis.rs` draws OS randomness
  (`getrandom`), which contradicts `"capabilities": []` for the analyzer, while
  declaring `rand:read:*` breaks the jail's `rand-syscall` negative control
  (RCG009) and the report's CWE-330 row. No source code changed.

## [0.1.0] - 2026-04-10

### Added

- DT17 hash-functions port in Rust
- FNV-1a 32-bit and 64-bit implementations with known-vector coverage
- DJB2 and polynomial rolling hash implementations
- MurmurHash3 32-bit implementation with seed support
- SipHash-2-4 implementation with 128-bit key support
- `HashFunction` trait plus concrete strategy structs for composition-style use
- `avalanche_score` and `distribution_test` analysis helpers
- Deterministic tests for the analysis helpers and known-vector tests for all algorithms
