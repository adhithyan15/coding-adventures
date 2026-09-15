## 0.66.0 — 2026-06-12 — Brainfuck runs on WASM (LANG-MATRIX LM-W Brainfuck)

`lang_matrix.rs` greens **Brainfuck on the WASM backend** — the last code-gen gap in
Brainfuck's row (only the VM/JIT columns remain). Verified by RUNNING
`++++++++[>++++++++<-]>+.` on the in-repo `wasm-runtime`: it prints `A`.

The lowering work is in `iir-to-wasm` 0.13.0 (byte-tape ops + i64↔i32 conversions for
the widened Brainfuck value model — see that crate's changelog). This crate's change is
in the test harness:

- `run_wasm`'s host (`PrintHost`) now also resolves Brainfuck's I/O imports:
  `env.putchar : (i32) -> ()` → a new `PutcharFunc` that captures raw output **bytes**
  (so `.` of cell value 65 yields stdout `A`, not the decimal `65` that `__print_i64`
  would), and `env.getchar : () -> i32` → a `GetcharFunc` returning EOF (`-1`).
- `run_wasm` prefers the byte stream when the program wrote any (Brainfuck), else the
  integer stream joined by newlines (BASIC) — the expression languages print nothing.
- `tests/lang_matrix.rs` adds `Wasm` to the Brainfuck `Prog`.

No `lang-aot/src` change: `lower_brainfuck_for_aot`'s i64 widening (added in 0.65.0 for
LLVM) is backend-agnostic and already flowed to the wasm path.

