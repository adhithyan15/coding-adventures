## 0.61.0 — 2026-06-11 — Expression languages join the JVM column (LANG-MATRIX LM-J)

`lang_matrix.rs` opens the **JVM** column on **real `java`** for the four expression
languages — Twig / Nib / Oct / ALGOL 60. New `Backend::Jvm` runner: source →
`compile_source_to_jvm_class` → the W16 wrapper-launcher (inject a
`main([Ljava/lang/String;)V` that invokes the entry `main()I` and
`System.out.println`s its `int`) → real `java` → parse the printed integer. Gated on
`java`, skipping gracefully when absent (mirrors the LLVM column's `clang` gate).

**Backend fix found by running on real `java`:** `concretize_scalar_any_for_jvm`
retyped a scalar function's return type and instruction hints to `i32` but left its
**parameter** types `i64`. Nib's `double(x)` therefore emitted the inconsistent method
signature `(J)I` with an `int`-computing body, which real `java` rejects with
`VerifyError: Expecting to find integer on stack` (the laxer in-repo `jvm-simulator`
used by `jvm_emit.rs` never caught it). The pass now concretizes parameter types too —
a reusable correctness fix, not a Nib-specific hack, and safe because lisp/`any`-param
functions are already skipped by the `uses_lisp` guard.

Verified by RUNNING on real `java` (OpenJDK 21): Twig→42, Nib→42, Oct→0,
ALGOL `17 mod 5`→2. **20 proven matrix cells** (was 16). `jvm_emit` (simulator floor),
`conformance` (McCarthy W16) and `wasm_emit` suites all still green. JVM column now green
for the expression languages; BASIC pends an `env.BasicRuntime` host class (its
`print_i64` → `invokestatic env/BasicRuntime.println(J)V`), Brainfuck the tape ops.

