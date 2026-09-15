## 0.30.0 — 2026-06-10 — McCarthy → **CLR symbols** on the simulator (LANG77 / W8a, F6)

Symbols (F6) run on the CLR with **zero new backend code** — pure structural-pass
reuse. The shared `intern_symbols_structural` pass interns each distinct symbol to
a stable `i32` id in a reserved range (`SYMBOL_ID_BASE = 1 << 29`); W6b boxing +
W7 `equal?`/`pair?`/`jmp_if` then execute `QUOTE`/`EQ`/`ATOM`/`COND` on symbols.
`(QUOTE A)`→536870912, `(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`→0,
`(ATOM (QUOTE A))`→1, on the `clr-simulator`. New `tests/cil_symbols.rs`. (The CLR
backend itself is unchanged — this release adds the regression test + the F6 tick.)
Remaining for CLR: **W8b lambda (F7)** — `call` lowering + simulator call frames.

