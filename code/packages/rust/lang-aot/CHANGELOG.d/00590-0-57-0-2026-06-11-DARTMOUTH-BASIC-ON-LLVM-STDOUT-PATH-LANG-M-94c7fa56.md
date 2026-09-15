## 0.57.0 — 2026-06-11 — Dartmouth BASIC on LLVM (stdout path) (LANG-MATRIX LM-L BASIC)

Greens **Dartmouth BASIC on LLVM**, the first I/O language in the LLVM column. The
`lang_matrix` `run_llvm` runner grew a stdout path: a BASIC program's `.ll` emits
`call void @__print_i64(i64 42)`, so when the emitted `.ll` references `@__print_i64`
the runner compiles in a tiny **generic** `__print_i64` C runtime (the LLVM analog of
wasm's `env.__print_i64` / JVM `BasicRuntime.println(J)V` / CLR `Console.WriteLine`)
and the harness compares the program's **stdout**. Verified by RUNNING: `10 PRINT 42`
→ stdout `42` on real `clang`. The LLVM column is now green for Twig / Nib / Oct /
ALGOL 60 / BASIC; only **Brainfuck** remains — it needs the tape ops
(`alloc_bytes`/`load_byte`/`store_byte` + `putchar`) added to `iir-to-llvm`, a backend
codegen slice. (Test-only change; the print runtime links only when a program
actually prints, so the bare expression-language cells are unaffected.)

