## 0.19.0 — 2026-06-09 — McCarthy → WebAssembly, **symbols** (LANG77 / W1)

`compile_source_to_wasm` now runs `intern_symbols_structural` (before the repr
pass), so McCarthy **symbols** (`QUOTE` / `'A`) work: each distinct symbol is a
distinct interned value (boxed as `i31ref`), so `(EQ 'A 'A)` → T, `(EQ 'A 'B)` →
nil, `(EQ 'A 5)` → nil. Symbols flow through cons cells and `COND` guards. With
this, the WASM backend runs the full McCarthy core **plus symbols** (F1–F6);
integer/cons/scalar programs are unaffected.

