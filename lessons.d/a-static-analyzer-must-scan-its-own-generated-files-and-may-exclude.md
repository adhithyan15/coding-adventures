---
category: Security boundaries
---

# A static analyzer must scan its own generated files and may exclude directory names only at the package root

The Rust capability analyzer skipped `src/capabilities.rs` ("to avoid
circularity") and skipped any directory named `target`, `node_modules` or
`_build` at any depth. Hand-editing the generated file, or writing
`mod target;` with `src/target/mod.rs`, hid arbitrary code from the analysis.
Other holes of the same family: a `use std::{fs, process};` group looked like a
bare `std`, unknown `std` paths defaulted to "pure", `extern "C-unwind"` was not
in a hard-coded ABI list, and `include!`, `#[path]`, proc-macros and
metavariable-spliced paths were never examined.

Fix: scan every `.rs` file (the pristine generated module scans clean, so no
exemption is needed), exclude build/test directories at the root only, expand
`use` trees and aliases package-wide, make unknown `std` surfaces and
unanalysable constructs a non-waivable `CAP003`, and pin each route in a
350-case evasion corpus with exact verdicts (`tests/test_evasion_corpus.rs`).

Do differently: an exclusion in a security scanner is a hole until proven
otherwise. Prefer scanning the excluded thing and making it clean; if an
exclusion is unavoidable, anchor it as narrowly as possible and add a test
that plants a violation inside the excluded path and expects it to be found
(or fail closed). Default unknown names to "deny", never to "pure".
