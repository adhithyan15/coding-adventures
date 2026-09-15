## 0.68.0 — 2026-06-12 — Brainfuck runs on CoreCLR — CODE-GEN MATRIX COMPLETE (LANG-MATRIX LM-C Brainfuck)

`lang_matrix.rs` greens **Brainfuck on real CoreCLR** — the **last code-gen cell** of the
LANG-PLATFORM-MATRIX. With it, **every language (Twig / Nib / Brainfuck / Dartmouth BASIC
/ Oct / ALGOL 60) runs on every code-gen backend (native-AOT / LLVM / WASM / JVM / CLR),
verified by running.** Only the deliberately-deferred VM and JIT columns remain. Verified
by RUNNING `++++++++[>++++++++<-]>+.` on `ilasm`+`dotnet`: it prints `A`.

The lowering work is in `iir-to-cil-bytecode` 0.18.0 (the textual `.il` byte-tape ops +
`putchar`/`getchar` + the launcher's `putchar`-aware "prints" detection — see that
crate's changelog). This crate's change is in the test harness: `tests/lang_matrix.rs`
adds `Clr` to the Brainfuck `Prog`. `run_clr` (assemble with real `ilasm` → run on real
`dotnet`) is unchanged — it already returns the captured `Console` output, so a
Brainfuck program's `Write(char)` byte stream (`A`) is compared against `Expect::Stdout`.

No `lang-aot/src` change: `lower_brainfuck_for_aot`'s rewrite + i64 widening (0.65.0) and
`concretize_scalar_any_for_cil` (which retypes Brainfuck back to `int32`, since it doesn't
call `print_i64`) already flowed to the CLR path.

