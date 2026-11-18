# Changelog

All notable changes to this package will be documented in this file.

## Unreleased — regenerated evidence harness (VOS-038b)

- `tests/generated/warrant_evidence.rs` regenerated with `gen_evidence` for the observer timeout clamp (`WARRANT_OBS_TIMEOUT_SECS` at most 24 h, `checked_add` deadline; VER04j). No test, case count or recorded evidence manifest changes.

## [Unreleased]

### Changed

- **WD-HARN1c: negative controls run through the shared checker.** All 9 sabotage lines in
  each of `BUILD`, `BUILD_linux` and `BUILD_windows` now call
  `code/scripts/warrant_negctl.py -- cargo test ...` instead of an `awk` pipe (Unix) or a
  `%TEMP%` capture plus `findstr` (Windows). Every fault, filter and cargo argument is
  unchanged and in the same order. The checker also requires a non-zero exit, exactly one TAP
  plan and test points numbered `1..N`, and prints the captured output when a control fails
  (the Windows lines printed nothing). Each BUILD file declares `# needs-toolchain: python`.
  No source, test or evidence change.

### Removed

- BREAKING: `avalanche_score` and `distribution_test` (module `analysis`) are no
  longer exported from this crate, and the `getrandom` dependency is gone. They
  moved, unchanged, to the new crate `coding_adventures_hash_analysis`
  (`code/packages/rust/hash-analysis`), because `avalanche_score` draws OS
  randomness and this crate is pure computation (WX-P1a2, DT17a section 7). No
  other package in the repository used them. Migration: depend on
  `coding_adventures_hash_analysis` and import them from there. The crate version
  is 0.2.0 (minor bump, public API removed).

### Added

- `steps` and `depth` `bound` rows on every operation of
  `code/specs/hash-functions.warrant` (WX-P1a2), so the report can address CWE-400.
  `heap <= 4096` on `polynomial_rolling`, `murmur3_32`, `murmur3_32_with_seed` and
  `siphash_2_4` (checked live under the allocation observer; observed peak 0 bytes).
  `polynomial_rolling_with_params` has no `heap` row: the zero-modulus path panics and
  allocates the std panic payload plus, on Linux and macOS, the unwinder's exception
  object (80 bytes Linux, 16 Windows; the unwinder is the likely explanation, not
  measured), so an exact committed peak would diff across platforms.  The 64 bytes a
  trial recorded on success paths were the evidence adapter's own `V::Ok(Box::new(..))`,
  not the crate.
  `fnv1a_32`, `fnv1a_64` and `djb2` have no `heap` row: their 2^24-case suites would
  cost about half an hour under the observer (see DT17a section 7). Evidence
  regenerated (`tests/generated/warrant_evidence.rs`, both manifests).
- `effect_certificate.json` (capability analyzer, label `proved`, no capabilities),
  `assurance-profile.json` (`non-cryptographic-hash`, kind `transform`) and
  `assurance_report.json`, with the analyzer and `ca-assurance-report --check`
  lines in `BUILD`, `BUILD_linux` and `BUILD_windows`. The report shows CWE-330
  excluded and CWE-400 and CWE-208 as unapproved exemptions (no live step-bound or
  constant-time proof); verdict `report-only`.

### Changed

- `required_capabilities.json` justification no longer mentions OS randomness (it moved
  to `hash-analysis`); `evidence.bindings` revision is 0.2.0; README gains a "Not
  cryptographic" note.
- The README usage example no longer imports the analysis helpers.

### Fixed

- README usage example: the SipHash-2-4 assertion `0x726f_db47_dd0e_0e31` is the
  published vector for the reference key `00 01 .. 0f`, but the example built an
  all-zero key (which gives `0x1e92_4b9d_7377_00d7`). The example now uses the
  reference key. The README's `fnv1a_64(b"abc")` assertion was also wrong (it was
  the value for `"a"`); the right value is `0xe71f_a219_0541_574b`. The README
  example is now compiled and run as a doctest (`#[cfg(doctest)]` include in
  `src/lib.rs`), so its assertions cannot drift again.

### Added

- Warrant adoption of the eight byte-string hashes (WX-P1a): the Warrant document
  `code/specs/hash-functions.warrant`, the contract
  `code/specs/DT17a-hash-functions-warrant.md`, `evidence.bindings`, the generated
  harness `tests/generated/warrant_evidence.rs` (277 tests, all passing: 73
  examples, 56 each of exhaustive, property and fuzz suites, 36 negative controls)
  and its manifest `evidence/test-manifest.json`, the shim
  `tests/warrant_evidence.rs`, and `evidence/proof-manifest.json` (written by
  `warrant-ir/tests/hash_functions_proofs.rs`; the `siphash_2_4` constant-time
  obligation is recorded `not-live`). This is the first harness that compiles the
  WX-FAULT1 fixed-width-integer arm of `flip-result-bit`.
- `tests/runtime_capability.rs` (VER08b): the eight covered hashes run on published
  and independently computed known answers inside a seccomp/Landlock jail derived
  from `required_capabilities.json` (no capabilities), watched by `strace`.
- `BUILD`, `BUILD_linux` (strict jail) and `BUILD_windows` run the unit and doc
  tests, the jail, `gen_evidence --check`, the evidence run and nine sabotage
  lines (eight `flip-result-bit`, one `missing-error`), then the proof manifest and
  a `git diff --exit-code -- evidence/` staleness check.
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
