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

Fix, second attempt: the *length* comes from Rust's shortest `{:e}`, which
is right about length. The *digits* come from `format!("{:.*e}", n - 1, x)`,
which is correctly rounded and half to even, when that spelling reads back as
`x`; otherwise they come from `{:e}` itself. The first fix searched for the
fewest correctly rounded digits that round-trip
(`(0..17).map(|d| format!("{:.*e}", d, x)).find(..)`). It rested on the claim
"if any p-digit spelling round-trips, the nearest one does", and that claim is
false at exact powers of two. There the round-trip interval is lopsided, so
the nearest 16-digit spelling of `0x0060000000000000` misses while
`7.120236347223045e-307` reads back. A review found 46 such powers of two that
disagreed with `repr`. A test now checks all 2,098 powers of two against host
`python3`.

Do differently: when a hand-written float oracle replaces typed expectations,
check it against an independently generated table first (here the
python3-generated `FLOAT_DIGITS_TABLE`), and make sure that table holds an
exact tie *and* an exact power of two (the lopsided interval). Sweep every
power of two against the host when you can.
