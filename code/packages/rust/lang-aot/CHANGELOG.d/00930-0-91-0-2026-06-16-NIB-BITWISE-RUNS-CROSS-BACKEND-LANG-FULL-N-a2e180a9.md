## 0.91.0 — 2026-06-16 — Nib bitwise `~` runs cross-backend (LANG-FULL N3)

`tests/lang_matrix.rs` gains two executed `~` programs proving unary bitwise NOT runs
on **all 7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT):

- `~0u8` — `let x: u8 = ~0; if x == 255 { return 1; }` → exit `1`. `~0` flips all bits;
  masked to u8 it is `255`. The `== 255` guard distinguishes the masked complement from
  an unmasked `not 0` (`-1`, which would give exit `0`).
- `~15u4` — `let x: u4 = ~15; if x == 0 { return 1; }` → exit `1`. On a nibble `~15 = 0`;
  proves the mask is *width-correct* (a u8/i64 mask would leave `0xF0`/`-16`, not `0`).

Driven by `nib-iir-compiler` 0.16.0 (lowers `~` → IIR `not` with the narrow width — it had
been silently dropped) and `iir-to-cil-bytecode` 0.21.0 (adds the unary `not` arm to its
textual `.il` emitter — the last backend that couldn't assemble `~` on CoreCLR). Completes
Nib N3 (`& | ^ ~`).

