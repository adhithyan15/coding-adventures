## Unreleased — LANG83 PR6c non-trapping IEEE f64 arithmetic and `f64_signbit`

- LANG83 PR6c — `f64_add`, `f64_sub`, `f64_mul`, `f64_div` and `f64_neg` (IEEE-754 binary64, never a trap) on the eight IEEE backends, and `f64_signbit` (0/1) on all nine. BEAM declares the five arithmetic ops `Refused`, so `check_module_for` and `compile_iir_to_beam` reject a module using them with `UnsupportedOps` at compile time. `tests/op_conformance.rs` gains three cases:
  - the spec's edge table: `1/0`, `-1/0`, `1/-0`, three NaNs, `1e308 + 1e308`, the signed-zero products and sums, `f64_neg` of `±0` and `-inf`, and `0.1 + 0.2`, each printed with `write_f64_js` and (except the NaNs, whose sign is unspecified) its sign bit with `write_i64`;
  - `f64_signbit` of finite constants (`±0`, `±2.5`, the smallest subnormals, `±max`) — the one PR6c case BEAM lowers;
  - every op with an aliased destination (`x = f64_add x, x`, …).

  New gate `pr6c_ops_follow_the_ieee_contract`: it pins the support matrix, requires all three cases on WASM, VM, JIT, LLVM, native and JVM whenever their tools exist (locally all six run), and checks that BEAM refuses the two arithmetic cases and compiles the sign-bit case. CLR, BEAM and AArch64 are structural here.
