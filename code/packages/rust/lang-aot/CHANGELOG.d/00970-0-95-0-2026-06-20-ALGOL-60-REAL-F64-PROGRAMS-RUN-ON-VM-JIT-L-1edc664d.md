## 0.95.0 — 2026-06-20 — ALGOL 60 real (f64) programs run on VM/JIT (LANG-FULL AL1 / E3 phase 1)

`tests/lang_matrix.rs` adds two ALGOL 60 **real** programs, executed on the VM
and JIT:

- `r := 2.5 * 2.0; if r = 5.0 then result := 42 …` → exit **42** (real multiply
  + `f64` equality), and
- `r := 7.0 / 2.0; if r < 4.0 then result := 1 …` → exit **1** (real division +
  `f64` ordered comparison).

These RUN end-to-end real arithmetic (`algol-iir-compiler` 0.4.0 lowers `real`
→ IIR `f64`; `vm-core` 0.6.0 executes the `f64` ops). The comparison fold yields
an integer exit code, so no float *printing* is needed to verify.

**Scope.** The proofs run on the VM + JIT only — they carry a tagged float value
model. The five code-gen backends are deferred E3 follow-ups: `iir-to-{llvm,
wasm,jvm}` model every variable slot as a uniform `i64` so an `f64` variable
can't be stored (E3-codegen-slots), and `iir-to-cil-bytecode` / the x86_64 +
aarch64 native backends reject `Operand::Float` (E3-clr / E3-native). See
`code/specs/LANG-FULL-IMPLEMENTATION.md`.

