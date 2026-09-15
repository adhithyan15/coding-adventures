## 0.216.0 - 2026-07-14 (AOT00 T7 — full-value differential via BASIC PRINT)

Adds `t7_differential_random_basic_print_agree` to `tests/lang_matrix.rs` — a
second, **strictly stronger** generative differential (AOT00 T7). Where the seed
`t7_differential_random_u8_expressions_agree` compares an 8-bit exit code (blind to
the upper bits — a `u8 200+100` reads 44 on every engine only because the OS
truncates the exit code), this generates `10 PRINT <expr>` Dartmouth BASIC programs
over `+ - *` and compares the **full `i64` value** printed on stdout — negatives and
large products included — across every engine, VM as the reference oracle. Literals
`0..=16`, depth ≤ 3 (all-`*` ≤ `16^8 ≈ 4.3e9`, inside `i64`, never overflows; no
division, so total). 368 full-value agreements over 160 programs locally; fast
in-process engines every program, toolchain engines (native/LLVM/CLR) sampled.
Spec: `code/specs/lang-full-t7-differential-harness.md` §3b.

