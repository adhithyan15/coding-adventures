## 0.48.0 — 2026-06-11 — McCarthy on **real CoreCLR** (scalar) — CLR real-runtime verification (CLR-real C1)

New `compile_source_to_cil_text(language, source, name) -> String`: the CLR analog
of `compile_source_to_llvm`. Where `compile_source_to_cil_artifact` yields raw method
bodies for the in-repo `clr-simulator`, this emits textual `.il` (via
`iir-to-cil-bytecode` 0.11.0) that real `ilasm` assembles into a loadable PE running
on real `dotnet`. New e2e test `tests/clr_real_scalar.rs` (gated on `dotnet`+`ilasm`,
skips when absent): `42`→42, `0`→0, `7`→7 on **real CoreCLR**. First slice of bringing
the CLR backend up to the same real-runtime verification bar as JVM/BEAM/LLVM/native.

