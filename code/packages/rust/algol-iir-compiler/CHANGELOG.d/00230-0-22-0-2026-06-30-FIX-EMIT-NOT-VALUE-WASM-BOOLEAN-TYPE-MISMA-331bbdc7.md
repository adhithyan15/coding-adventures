## 0.22.0 — 2026-06-30 — Fix `emit_not_value` WASM boolean type mismatch

`emit_not_value` implements `not b` as `cmp_eq(b, false_const)`.  The
`false_const` slot was emitted with `ScalarType::Integer` and
`Operand::Int(0)`, which caused `infer_local_type_hints` in the WASM
backend to assign the local a type of "i64".  When `b` is a boolean
variable (local type "bool", i.e. i32), the subsequent `cmp_eq`
instruction selected `I32_EQ` but one operand was i64 → WASM validation
failure.

Fix: change `false_const` to use `ScalarType::Boolean` + `Operand::Bool(false)`.
The WASM backend then assigns type "bool" (i32) to the false_const slot,
matching the boolean variable's i32 local.  LLVM is unaffected — both
`Operand::Int(0)` and `Operand::Bool(false)` lower to the literal `0` in
LLVM IR.

