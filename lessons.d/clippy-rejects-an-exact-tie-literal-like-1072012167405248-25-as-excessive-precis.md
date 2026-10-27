---
category: Rust
---

# Clippy rejects an exact tie literal like 1072012167405248.25 as excessive precision; write the shortest spelling that parses to the same double

**What went wrong.** LANG83 PR7b's LLVM and WASM tests listed the tie value
`1072012167405248.25` (exactly representable; ulp 1/8) as an `f64` literal.
Every test passed, but `cargo clippy --all-targets -D warnings` failed with
`float has excessive precision`. The shortest decimal that parses to that
double is `1072012167405248.2`, and clippy's `excessive_precision` lint
flags any literal with more digits than needed, even when it is exact.

**Fix.** Write `1072012167405248.2`, which is the same `f64`. Or write the bit
pattern (`f64::from_bits(0x430E77EA4376F602)`) when the exact value is the
point of the test.

**Do differently.** Float tables in tests should use the shortest spelling
(Python `repr`) or bit patterns. Run clippy with `--all-targets` before you
call a slice done, because `cargo test` does not run lints.
