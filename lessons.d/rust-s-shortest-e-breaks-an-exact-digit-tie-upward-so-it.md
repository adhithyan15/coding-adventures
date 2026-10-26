---
category: Rust
---

# Rust's shortest {:e} breaks an exact digit tie upward, so it is not a repr oracle

A test oracle for LANG83's `f64_digits` / `f64_exp10` (op_conformance's
`host_float_parts`) first split Rust's shortest round-trip `format!("{:e}", x)`
at the `e`. It failed on one row of the PR7b digits table: `2⁻²⁵`, whose exact
value `2.98023223876953125e-8` lies exactly halfway between the two shortest
17-digit spellings. Rust's shortest `{:e}` gives `…695313e-8`; Python's `repr`
(David Gay's mode 0) and vm-core's digits give the correctly rounded
`…695312e-8`, half to even. Both round-trip, so a round-trip check cannot
tell them apart.

Fix: take the fewest significant digits whose *correctly rounded* spelling
round-trips, i.e. `(0..17).map(|d| format!("{:.*e}", d, x)).find(|s|
s.parse() == Ok(x))`. Rust's fixed-precision formatting is exact and rounds
half to even, and if any p-digit spelling round-trips the nearest one does,
so this matches `repr` digit for digit.

Do differently: when a hand-written float oracle replaces typed expectations,
check it against an independently generated table first (here the
python3-generated `FLOAT_DIGITS_TABLE`), and make sure that table holds an
exact tie.
