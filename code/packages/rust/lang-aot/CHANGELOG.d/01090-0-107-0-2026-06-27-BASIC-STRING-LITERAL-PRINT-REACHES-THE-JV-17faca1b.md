## 0.107.0 — 2026-06-27 — BASIC string literal PRINT reaches the JVM column (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **JVM + CLR + VM + JIT**.
This extends the 0.106.0 CLR foothold to a second managed code-gen backend:
the JVM backend maps `str_const` to `ldc` + `CONSTANT_String` and `print_str` to
`PrintStream.print(String)`. The existing BASIC newline still flows through
`putchar`.

This is still the literal-output shape only; `str_len`, `str_index`, `str_concat`,
and `str_eq` fail closed until the managed backends own byte-string semantics.
WASM/native/LLVM string lowering remain follow-up slices.

