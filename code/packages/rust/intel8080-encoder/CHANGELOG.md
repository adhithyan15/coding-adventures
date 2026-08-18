# Changelog — intel8080-encoder

## v0.1.0 — 2026-08-17 — third lane of the 9-architecture expansion

Initial release. Re-exports `encode_mvi_a`, `assemble`, `HLT`, `RET`, and
the 8080 register-code constants (`REG_A`..`REG_M`) from
`intel8080-simulator`. Enough for `intel8080-backend`'s minimal-viable
`const_*`/`ret_*` scope.

9 unit tests pin the re-exported constants and the canonical
`const 42; ret` byte derivation.
