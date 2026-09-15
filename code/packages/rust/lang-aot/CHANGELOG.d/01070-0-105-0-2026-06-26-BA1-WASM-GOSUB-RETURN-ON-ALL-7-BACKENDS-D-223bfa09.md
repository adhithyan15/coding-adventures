## 0.105.0 — 2026-06-26 — BA1-WASM: GOSUB/RETURN on all 7 backends (dispatch-loop fix)

**LANG-FULL BA1** — both executed Dartmouth BASIC `GOSUB`/`RETURN` programs
now run on all **7** backends (native-AOT + LLVM + **WASM** + JVM + CLR + VM + JIT):

- `10 GOSUB 100 / 20 PRINT 1; / 30 GOSUB 100 / 40 END / 100 PRINT 9; / 110 RETURN`
  ⇒ stdout `919` — the same `RETURN` resumes at two different `GOSUB` sites,
  proved on WASM for the first time.
- `10 GOSUB 100 / 20 END / 100 PRINT 8; / 110 GOSUB 200 / 120 PRINT 6; / 130 RETURN /
  200 PRINT 7; / 210 RETURN` ⇒ stdout `876` — nested depth-2 LIFO discipline.

The fix is in `iir-to-wasm` 0.18.0: when the last basic block contains
`jmp_if_true`/`jmp_if_false`, a sentinel empty block is appended so the dispatch
chain is in the second-to-last block, where the loop-restart depth formula works
correctly.  The WASM `StackUnderflow` trap is eliminated.

