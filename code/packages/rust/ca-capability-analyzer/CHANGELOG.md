# Changelog

## Unreleased - Round-3 security review fixes (VER08a divergence D25 onwards)

Each fix has a regression in `tests/test_secreview_round3.rs` built from the
reviewer's proof-of-concept crate, which printed `PASS` while `cargo check`
compiled the capability in.

- [false negative, HIGH-3] A file starting `#!/x"` (a shebang line rustc ignores) opened a string in the analyzer that swallowed the following code. The lexer now applies rustc's `strip_shebang` rule exactly (after a leading BOM: `#!` is a shebang line unless, after whitespace and non-doc comments, the next token is `[`).
- [false negative, HIGH-6] Whitespace was `char::is_whitespace`, not Rust's `Pattern_White_Space`: `std::fs::<U+200E>write` read as `std::fs` (PoC t9). Identifiers used `is_alphanumeric`, not XID: `extern crate std as s·x;` (U+00B7) bound `s` in the analyzer and `s·x` in rustc (PoC t4). Whitespace is now exactly `Pattern_White_Space`; any other non-ASCII character outside a literal or comment is lexed into an identifier and is `CAP003` (no XID/NFC tables in a zero-dependency crate); a byte-order mark is skipped only at offset 0. `extern crate NAME [as ALIAS]` must be followed by `;`, otherwise `CAP003`.
- [false negative, HIGH-7] `c"`/`cr#"` were lexed as C strings in every edition. In an edition-2018 crate `cr#" a " { std::fs::write(..) } " "#` is `cr`, `#`, a string, a block and a string, so the block is code (PoC t8). `Cargo.toml` `package.edition` is now read first (`CargoFacts::edition`) and passed to the lexer (`scan_sources_with_edition`, `Edition`): C-string prefixes exist only from 2021. An unknown edition (missing, `edition.workspace = true`, a per-target `edition` key, a future value, unparseable `Cargo.toml`) makes a raw C-string prefix `CAP003`; a cooked `c"` has the same boundaries either way and is not flagged.
- [false negative, HIGH-4] `include!` was matched only as `include!` or `std::include!`; the preludes re-export it and `prelude` is a pure module, so `std::prelude::v1::include!("payload.txt")` passed (PoC t3). Any `std`/`core`/`alloc` path ending in `include`, at any length, is now `CAP003`, including the `use` that imports it (`use std::prelude::v1::include as i;`). `include_str!`/`include_bytes!` stay pure (data, L-B).
- [false negative, HIGH-1] A glob re-exported through a local module was spelled only as `glob + <whole path>`: with `mod m { pub use std::*; }`, `m::process::Command::new(..)` became `std::m::process::Command` (unknown std module, whose `CAP003` glob spellings suppressed) and passed (PoC t12); with `mod m { pub use std::fs::*; }`, `m::remove_dir_all` became `std::fs::m::remove_dir_all`, i.e. only `fs:read` (PoC t13). Glob spellings are now `glob + suffix` for *every* suffix of every alias spelling (crate-/self-rooted paths included), glob paths are normalised through bindings at every segment (`use crate::s::*` with `pub use std as s`), a glob over a bare `std`/`core`/`alloc` root (`use std::*`, `use s::*` after `use std as s`) is `CAP003`, and `CAP003` from glob spellings is no longer suppressed when the glob prefix is such a bare root.
- [denial of service, LOW] Glob normalisation materialised (globs x binding targets) and counted only the globs it kept. Every spelling, pure or not, now counts against a 4,096 cap checked inside the loop; exceeding it is `CAP003`. Glob spellings in the walk are charged to the per-file 200,000-spelling budget.

## 0.3.2 - Round-2 security fixes (VER08a divergence D22-D24)

- [false negative, N1] `macro_rules! ls { ($t:ty, $p:expr) => { <$t>::read_dir($p) } }` (also `<$t>::exists`, `try_exists`, `read_link`, `canonicalize`, `<$t as Foo>::exists`) hid `Path` UFCS. A `$` metavariable inside a `<..>` group followed by `::` in a `macro_rules!` body is now `CAP003` (general rule, not a method list).
- [false negative, N2] `$($t)*::read_dir` / `$($t)+::` / `$($t),*::` evaded the repetition rule. Any `$(..)` repetition, with or without separator and kleene operator, followed by `::` is now `CAP003`.
- [false negative, L-A] `Cargo.toml` unscanned-root comparison (`tests`, `benches`, `examples`, `target`, ..) is now ASCII case-insensitive (`path = "Tests/evil.rs"`).
- [false-positive guard] `<$t>::MAX` (SCREAMING_CASE associated constant) is exempt from the N1 rule; `segment-tree` uses it and a constant cannot do I/O.
- [docs, L-B] VER08a documents why `include_str!`/`include_bytes!` are not flagged.

## 0.3.1 - Security-review fixes (VER08a divergence D14-D21)

Specs first: VER08a updated (rules §3.2-3.5.1, work ceilings §5, divergence log D14-D21).
Every fix has a regression that fails on 0.3.0 (corpus group `SECFIX` in
`tests/test_evasion_corpus.rs`; `tests/test_secfix_regression.rs`).

- [false negative, M1] `Path::exists(p)`, `std::path::Path::metadata(p)`, `PathBuf::read_dir(p)`, `<PathBuf>::is_file(p)` and `let g = Path::canonicalize;` were pure. Any `…::Path/PathBuf::<fs method>` is now `fs:list`; `std::path::absolute` is `env:read`. The path reader understands `>::item` and turbofish.
- [false negative, M2] `m!(include, "../tests/evil.rs")` through `macro_rules! m { ($mac:ident, $f:expr) => { $mac!($f) } }` evaded `CAP003`. A bare `include`/`include_str`/`include_bytes` argument of a macro invocation and any `$name!(..)` in a `macro_rules!` body are now `CAP003`.
- [false negative, M3] The line-oriented `Cargo.toml` reader missed top-level inline tables, `bin = [{ path = .. }]`, `package = { build = .. }`, `lib = { proc-macro = true }`, quoted table names, multi-line strings and multi-line arrays that reset the "current section". Replaced by a tokenizing reader that flattens every value to a key path, flags `path`/`build`/`proc-macro`/`proc_macro` anywhere outside dependency tables, refuses unverifiable values (multi-line strings, escapes, tables, arrays) and fails closed on anything it cannot parse. A line the old reader ignored (garbage without `=`) is now a finding.
- [denial of service, M4] Alias expansion materialised (entries x targets) before checking its cap (264 KB input -> 2.3 GB abort); candidate generation was quadratic in path length; `matching_close` rescanned from every `#[` (O(n^2) on `#[#[#[…`); glob count was uncapped. Now: de-dup before expansion and caps inside the loops, 64 segments / 64 spellings per path, per-file work budget, a single-pass bracket table, 256-glob cap, macro-body scan budget. All reviewer inputs finish in well under a second in a debug build and yield `CAP003`.
- [false negative, L1] `type F = File; F::create(..)`, `<File>::create(..)`, `type U = UdpSocket; U::bind(..)` escalated within a category. `type` aliases are bindings and `<T>::item` is classified as `T::item`.
- [false negative, L2] `core::arch`/`std::arch` `_rdrand*`/`_rdseed*` intrinsics are `rand:read` (`__cpuid` documented as informational).
- [false negative, L3] `extern "Rust" { … }` blocks are `foreign_linkage` + `ffi:call`.
- [false positive, L4] The `.canonicalize()`-style method heuristic now resolves the one receiver shape it can prove (`Local::ctor(..)[.unwrap()/.expect()/.clone()/?]` with an inherent method of that name on a package-local type). `url-parser` no longer needs a false `fs:list` declaration; its manifest is corrected to `capabilities: []`.
- [behaviour] New type-alias bindings mean `type Handle = *mut std::ffi::c_void;` makes each later use of `Handle` an `ffi:call` detection, as the `use` form already did (packages already failing the analyzer: audio-device-coreaudio, iocp, objc-bridge, transport-platform).

Tests: 68 new SECFIX corpus cases (each with decoys), 17 regression tests (M3 bypasses and legitimate manifests, M4 timings).

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
