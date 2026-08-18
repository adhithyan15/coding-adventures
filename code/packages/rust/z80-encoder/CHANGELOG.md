# Changelog — z80-encoder

## v0.1.0 — 2026-08-17 — seventh lane of the 9-architecture expansion

Initial release. Re-exports `encode_ld_a_n`, `assemble`, `HALT`, `RET`,
and `REG_A` from `z80-simulator`. Enough for `z80-backend`'s
minimal-viable `const_*`/`ret_*` scope.

`encode_ld_a_n`/`HALT` are byte-identical to
`intel8080_encoder::encode_mvi_a`/`intel8080_encoder::HLT`.

8 unit tests pin the re-exported constants, the canonical `const 42; ret`
byte derivation, and the intel8080-encoder byte-identity claim.
