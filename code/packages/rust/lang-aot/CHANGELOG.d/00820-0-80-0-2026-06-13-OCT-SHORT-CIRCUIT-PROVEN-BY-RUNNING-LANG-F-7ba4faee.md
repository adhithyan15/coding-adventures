## 0.80.0 — 2026-06-13 — Oct `&&` / `||` short-circuit, proven by running (LANG-FULL O1)

`tests/lang_matrix.rs` gains two executed Oct programs that PROVE short-circuit via
a side-effecting function call in the right operand — `if 1 == 2 && side() == 1 { … }
else { out(1, 9) }` where `side()` prints 5 → stdout **"9"** (the old eager code
printed "5","9"), and the `||` analogue → **"7"** — across native / LLVM / WASM /
CLR / VM / JIT (JVM excluded: branch + print, the BA-JVM-1 follow-up). Backed by
`oct-iir-compiler` 0.6.0 (short-circuit lowering + the i64 function-return fix the
proof's `side() -> u8` helper exposed). No `lang-aot/src` change.

