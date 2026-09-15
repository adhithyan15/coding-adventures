## 0.103.0 — 2026-06-26 — Dartmouth BASIC multi-item `PRINT` on all 7 backends (LANG-FULL BA2)

`tests/lang_matrix.rs` gains two executed Dartmouth BASIC programs proving
**same-line multi-item `PRINT`** across native-AOT + LLVM + WASM + JVM + CLR +
VM + JIT:

- `10 PRINT 0 - 12; 34` ⇒ stdout **`-1234`** — two items joined tightly by `;`
  on one line, with multi-digit rendering (`12`, `34`) and a negative sign. The
  old per-item `print_i64` lowering appended a newline, so this would have been
  `-12⏎34`; BA2's character-level model (`dartmouth-basic-iir-compiler` 0.9.0)
  renders digits via the universal `putchar` builtin so items share a line.
- `10 PRINT 5, 6` ⇒ stdout **`5 6`** — the `,` separator inserts a space (vs
  `;`'s tight join).

Both reuse only ops the matrix already runs everywhere (`call`, integer
arithmetic, `cmp_*`, `putchar`), so BA2 added **zero** backend changes. The
existing single-item BASIC cells now route through `putchar` as well and still
pass unchanged. Bumped to 0.103.0.

