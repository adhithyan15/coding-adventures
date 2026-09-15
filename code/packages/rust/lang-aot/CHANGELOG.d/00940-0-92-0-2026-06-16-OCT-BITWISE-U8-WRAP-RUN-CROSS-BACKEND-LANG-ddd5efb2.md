## 0.92.0 — 2026-06-16 — Oct bitwise `~` + u8 wrap run cross-backend (LANG-FULL O2)

`tests/lang_matrix.rs` gains two executed Oct programs proving u8 width semantics run on
**all 7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT):

- `out(1, ~0)` → `255`. Oct's only integer type is `u8`, so `~0` flips 8 bits → `255`
  (`-1 & 0xFF`), not the i64 all-ones. (An unmasked `~0` would print `-1`.)
- `out(1, 200 + 100)` → `44`. The grammar specifies Oct addition wraps mod-256; `300` wraps
  to `44`. Distinct from the existing `100 + 100 = 200` Oct program, which does not overflow
  — this one proves the wrap fires.

Driven by `oct-iir-compiler` 0.7.0 (emits the `u8` hint on arithmetic/bitwise/`~`; Oct has a
single integer width, so every integer op is u8) and `iir-to-jvm-class-file` 0.14.0 (Oct's
`out` programs keep the JVM long model, so the narrow mask had to become `i2l; land` over the
long result — the int `iand` was unverifiable and returned empty on java). Completes Oct O2.

