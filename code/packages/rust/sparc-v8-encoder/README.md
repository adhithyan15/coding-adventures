# sparc-v8-encoder

Pure-Rust SPARC V8 instruction encoder.  Mirror of `mips-r2000-encoder`
/ `arm1-encoder`.

Sixth lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## What's inside

* Re-exports of `encode_add_imm`, `encode_ta`, `assemble` from
  `sparc_v8_simulator` (the in-tree source of truth for SPARC V8 bit
  layout).
* Register-role constants: `G0` (hardwired zero) and `O0` (the SPARC
  calling-convention return-value register — see `src/lib.rs`'s
  crate-level doc comment for why `%o0`, not a `%g` register, is safe
  to use here).
* Canonical word constant: `HALT_WORD = 0x91D0_2000` (i.e. `ta 0` —
  trap always, software trap #0 — the HALT sentinel
  `sparc-v8-simulator` intercepts to stop execution).

No IR knowledge.  `sparc-v8-backend` is the consumer that maps CIR ops
onto encoder calls.
