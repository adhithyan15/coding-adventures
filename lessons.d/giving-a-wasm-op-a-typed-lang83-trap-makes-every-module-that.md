---
category: Compiler / VM / language pipeline
---

# Giving a WASM op a typed LANG83 trap makes every module that uses it import env.__trap, so every test host that runs the op must provide that import

**What went wrong.** `iir-to-wasm` lowered `real_to_int_trunc` /
`real_to_int_floor` to a bare `i64.trunc_f64_s`, whose own trap names no
LANG83 kind. To raise the typed `Trap.ConvRange`, the lowering gained a range
check that calls `env.__trap(4)`, and `lang83_uses_trap` was extended so a
module with a narrowing registers that import. The three runtime tests in
`iir-to-wasm/tests/e8_conversions.rs` then failed on in-range inputs with
`unknown import: env.__trap`. They ran on `WasmRuntime::new()`, a host with
no imports at all. That was enough until the op needed one.

**Why.** A WASM import is resolved when the module is instantiated, not when
the call runs. A host that lacks the import cannot run *any* program that uses
the op, including programs that never fail.

**Do instead.** When an op starts to trap with a LANG83 kind on WASM, grep
for every test that instantiates modules using that op
(`WasmRuntime::new()`, `load_and_run`, custom `HostInterface`s), and give each
host an `env.__trap(i32)` import. `lang-aot`'s `op_conformance` and
`lang_matrix` hosts already provide it, because they run `Trap.DivZero`
programs. Crate-local e2e tests often do not.
