## 0.67.0 — 2026-06-12 — Brainfuck runs on the JVM (LANG-MATRIX LM-J Brainfuck)

`lang_matrix.rs` greens **Brainfuck on the JVM backend** — the last code-gen gap in
Brainfuck's row (only the deferred VM/JIT columns remain). Verified by RUNNING
`++++++++[>++++++++<-]>+.` on real `java`: it prints `A`.

The lowering work is in `iir-to-jvm-class-file` 0.11.0 (byte-tape ops + i64 branch
conditions — see that crate's changelog). This crate's change is in the test harness:

- New `BF_RUNTIME_JAVA` host class (`env.BFRuntime`) — a static `byte[] __tape` (the
  30 000-cell tape `alloc_bytes` references), `putchar(int)` (writes a raw byte to
  stdout — so `.` of 65 yields the byte `A`, not the decimal `65` that BASIC's
  `println` would), and `getchar()` (returns `0` at EOF).
- `run_jvm` compiles the host class the program's I/O lowers to: `env.BFRuntime` for
  Brainfuck, `env.BasicRuntime` for Dartmouth BASIC. The existing discard-launcher
  (run the entry, `pop`/`pop2` its result) handles Brainfuck's `int` exit unchanged;
  the captured `System.out` bytes are the program's stdout.
- `tests/lang_matrix.rs` adds `Jvm` to the Brainfuck `Prog`.

No `lang-aot/src` change: `lower_brainfuck_for_aot`'s rewrite + i64 widening (added in
0.65.0 for LLVM) is backend-agnostic and already flowed to the JVM path.

