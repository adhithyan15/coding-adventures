## 0.98.0 — 2026-06-20 — ALGOL 60 reals now run on the JVM too — E3-codegen-slots complete (LANG-FULL E3-codegen-slots, JVM)

The two ALGOL 60 **real** matrix programs now also run on the **JVM** column
(LLVM + WASM + JVM + VM + JIT — 5 of 7 backends). `iir-to-jvm-class-file` 0.15.0
fixes the two remaining f64 gaps: non-0/1 double constants now get a real
`CONSTANT_Double` pool entry (`ldc2_w <idx>` instead of the invalid `#0`
placeholder), and f64 comparisons emit `dcmpl`/`dcmpg` + a unary branch instead
of falling through to the integer `if_icmp` path (which mis-read a two-slot
double as an int → VerifyError). The `proven_columns_do_not_silently_skip`
guard confirms real `java` executes the programs.

**This completes E3-codegen-slots** (LLVM + WASM + JVM all run reals). Remaining
E3: the two direct native backends (x86_64/aarch64 — E3-native) and CLR
(E3-clr) still reject `Operand::Float`.

