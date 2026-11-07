## Unreleased — Twig floored modulo lowers to `i64_mod_floor` (#18486)

`dynamic_arith` now lowers `modulo` to `i64_mod_floor` rather than truncating `mod`.
`twig-aot` maps `modulo` to `i64_mod_floor`, `remainder` to `mod`, and `quotient` to `div`.
`tests/e6d2b_dynamic_arith.rs` adds `modulo_floors_and_remainder_truncates`, asserting that `(modulo -7 2)` → 1, `(modulo 7 -2)` → -1, `(remainder -7 2)` → -1, and `(modulo 6 3)` → 0 match across VM, JIT, and LLVM/NativeAot.
