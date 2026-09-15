## 0.93.0 — 2026-06-17 — Brainfuck cat `,[.,]` runs cross-backend (LANG-FULL B1-eof)

`tests/lang_matrix.rs` adds the canonical Brainfuck **cat** — `,[.,]` (read a byte and,
while non-zero, print it and read the next) — with input `"Hi"` → stdout `"Hi"`, on **all
7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT).

This closes the EOF-convention divergence deferred from B1-stdin (#6006). The `,+.`/`,.,.`
stdin programs read exactly their input and never hit EOF; cat reads *past* the input, so
it exercises the convention directly. Backends disagreed — JVM/VM/JIT returned `0` at EOF,
but libc `getchar`/`Console.Read`/the wasm host returned `-1` → the u8 cell wrapped to
`255` → cat looped forever there. `brainfuck-iir-compiler` 0.4.0 now clamps a negative `,`
result to `0` in the shared IIR (read at i64, `cmp_lt 0` + branch, store u8), so EOF is `0`
on every backend and cat halts. No per-backend change.

