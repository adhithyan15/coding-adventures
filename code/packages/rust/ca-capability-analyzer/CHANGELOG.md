# Changelog

## 0.3.0 - Audit hardening (VER08a / VER23a contract revision)

Specs first: `code/specs/VER08a-rust-capability-analyzer.md` and
`VER23a-rust-effect-modules.md` now state exact, testable contracts, the
out-of-scope limits of a lexical analyzer (G-1..G-9), and divergence logs
(D1-D13, V1-V10).

Bugs fixed (severity in brackets):

- [false negative] `use std::{fs, process::Command};` and every brace group were read as a bare `std` (pure). The scanner is now a real lexer plus a package-wide `use`/alias/glob/`extern crate` resolver.
- [false negative] `src/capabilities.rs` was never scanned; nested directories named `target`/`node_modules`/`_build` were skipped (`mod target;` hid code). Everything is scanned; exclusions apply at the package root only.
- [false negative] Unknown `std`/`core`/`alloc` paths defaulted to pure. They are now `CAP003` (deny by default).
- [false negative] Only six `extern` ABI names counted as FFI (`"C-unwind"` escaped); now every ABI except `"Rust"`. Attributes matched by substring; now exact identifiers, plus `link`, `link_section`, `link_ordinal`, `linkage`, `naked`.
- [false negative] `include!`, `#[path]`, proc-macro crates/attributes, metavariable-spliced paths and attributes in `macro_rules!`, and `Cargo.toml` paths into unscanned directories are now `CAP003` (never waivable).
- [false negative, AF-22] A clean report hid that `unsafe` was present under a manifest exception. Reports now list `excepted_constructs`, print them on every PASS, and end with a machine-readable `SUMMARY` line; `AnalysisReport::construct_status` distinguishes absent / present-excepted / present-violation.
- [false negative] `Path::exists()`-style filesystem probes (method heuristic), `RandomState`/rand crates (`rand` category), `std::os::*`, `std::backtrace`, `_print` and `dbg!` are now detected.
- [false claim] `--emit-cert` hard-coded `proved` without analysing; it now refuses unless the analysis is clean. Entry-point signatures were truncated at the first `;` (the committed ct-compare certificate was wrong and is regenerated). Entry effects of capability-bearing packages are now the sound over-approximation (all declared effects).
- [tamper] The certificate checker ignored `unconditional_roots`, exception justifications, unknown fields and duplicate keys, and matched entry points by name. It now recomputes the whole expected certificate and compares every field; JSON output is fully escaped; the digest walker refuses symlinks and exempts only the top-level `src/capabilities.rs`.
- [robustness] Manifest JSON: duplicate keys, lenient numbers/whitespace/control characters, duplicate or unknown banned-construct exceptions are rejected. Violations sorted by enum variant instead of file/line/col; now by location.
- [false positive] `std::time::Duration`, `std::net::IpAddr` and other data types are pure.

Tests: 350-case evasion corpus with exact verdicts, 224 hashlib-generated SHA-256 vectors plus NIST examples, 30 certificate tamper tests, 17 manifest tests, 34 package/scanner/Cargo/CLI tests (all in addition to the previous 42). Measured coverage 96.6% lines / 96.4% regions (`cargo llvm-cov`). 20 hand-made mutants, all killed.

Known limits (documented, not fixed): third-party dependencies are only reported; capability `target` is not enforced; `tests/`, `benches/`, `examples/` are not analysed; macros are not expanded.

## 0.2.0 — 2026-11-08

- Generated capability module and canonical effect certificate support (VER23 / VER24 / VER-P027b):
  - Sealed capability module generator (`src/capabilities.rs`, VER23) enforcing "you cannot name what you did not declare":
    - Pure computation packages receive `PureCapabilities` zero-effect token with `#![deny(unsafe_code)]`;
    - Capability-bearing packages receive typed witness handles strictly limited to declared capabilities;
    - `--generate-module` and `--check-module` for pre-commit generation and CI regeneration verification;
  - Canonical effect proof certificates (`effect_certificate.json`, schema `ver24-effect-cert-v1`, VER24):
    - Independent small certificate checker reading package sources, manifests, and certificates directly;
    - Cryptographic binding via deterministic multi-file SHA-256 source and manifest digests;
    - Verification of public entry points, declared effect bounds, unconditional roots, and honest `"proved"` label;
    - Comprehensive sabotage tests detecting source/manifest tampering, uncertified entry points, undeclared effects, and dishonest labels;
    - Self-contained FIPS 180-4 SHA-256 cryptographic hasher with zero external dependencies.

## 0.1.0 — 2026-11-08

- Initial implementation of the Rust Static Capability Analyzer (`ca-capability-analyzer-rs`, VER08 / VER08a / VER-P027a):
  - Zero third-party dependencies: implemented purely with the Rust standard library;
  - Strict `required_capabilities.json` schema validation against Spec 13 closed category/action taxonomy;
  - Automatic fallback to pure computation (`capabilities = []`) on absent manifest;
  - Deny-by-default ambient authority detection for filesystem (`fs`), networking (`net`), process execution (`proc`), environment access (`env`), system timing (`time`), standard streams (`stdin`/`stdout`), threads (`thread`), and foreign function interface (`ffi`);
  - Pure computation core allowlist (`core::*`, `std::vec::*`, `std::cmp::*`, `std::mem::*`, `core::hint::black_box`);
  - Banned construct inspection (`unsafe_code`, `inline_assembly`, `foreign_linkage`) with fingerprint exception support;
  - Resource quotas: 10,000 file cap, 4 MiB per-file limit, 64 MiB total source limit, 1 MiB manifest limit, symlink refusal;
  - Diagnostic formatting with stable exit codes (0 for pass, 1 for violations, 2 for tool errors).
