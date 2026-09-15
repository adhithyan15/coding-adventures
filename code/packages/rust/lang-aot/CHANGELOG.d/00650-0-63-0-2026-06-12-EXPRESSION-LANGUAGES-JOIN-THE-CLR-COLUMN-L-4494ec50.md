## 0.63.0 — 2026-06-12 — Expression languages join the CLR column (LANG-MATRIX LM-C)

`lang_matrix.rs` opens the **CLR** column on the **real CoreCLR** for the four
expression languages — Twig / Nib / Oct / ALGOL 60. New `Backend::Clr` runner: source →
textual `.il` (`compile_source_to_cil_text`) → real `ilasm` (`-exe`) → real `dotnet` →
parse the integer the entry `Console.WriteLine`s (the CLR-real path of the McCarthy
chapter, generalized). Gated on `dotnet` + a locatable `ilasm` (reuses
`clr_support::find_ilasm`'s NuGet-cache walk); skips gracefully when absent.

Two gaps surfaced **by running** on real `ilasm`/`dotnet`:
* **CIL backend missing arithmetic + comparisons.** `iir-to-cil-bytecode` rejected
  `add` (Nib), `cmp_eq` (Oct) and `mod` (ALGOL) — McCarthy only ever emitted a constant.
  Grown in `iir-to-cil-bytecode` 0.16.0 (CIL `add`/`sub`/`mul`/`div`/`rem` + `ceq`/`clt`/`cgt`).
* **`concretize_scalar_any_for_cil` parameter bug** (the CLR twin of the JVM fix): the
  pass retyped a scalar function's return + body to `i32` but left its **parameters**
  `i64`, so Nib's `double(x)` emitted the inconsistent CIL signature `int32(int64)` that
  CoreCLR's verifier rejects. The pass now concretizes parameter types too.

Verified by RUNNING on CoreCLR (dotnet 9.0.x): Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2.
**25 proven matrix cells** (was 21); the `conformance`, `jvm_emit`, `wasm_emit`,
`iir-to-cil-bytecode` suites all still green. CLR column now green for the expression
languages; BASIC pends a `Console`-writing `print_i64`, Brainfuck the tape ops.

