## 0.24.0 — 2026-06-09 — McCarthy **symbols** on the JVM (LANG77 / W5a, F6)

`compile_source_to_jvm` now produces working classes for McCarthy **symbols**:
`(EQ 'X 'X)` → 1, `(EQ 'X 'Y)` → 0, `(QUOTE X)` → its interned id, all run on a
real `java`. No lang-aot code change — the win is in `iir-to-jvm-class-file`
0.10.0, which fixed the large-`int` constant `ldc` path (a symbol id lives in the
`2²⁹` reserved range, too big for `bipush`/`sipush`; the old backend emitted an
invalid `ldc 0` that crashed the JVM). New `tests/jvm_symbols.rs` round-trips it
on a real JVM.

