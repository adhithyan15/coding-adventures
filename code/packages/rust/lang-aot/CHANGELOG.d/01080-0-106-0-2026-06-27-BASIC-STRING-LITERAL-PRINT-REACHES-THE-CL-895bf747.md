## 0.106.0 — 2026-06-27 — BASIC string literal PRINT reaches the CLR column (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **CLR + VM + JIT**.
This extends the E4/BA4 source-language proof beyond the interpreter columns:
the CLR textual `.il` backend maps `str_const` to `ldstr` and `print_str` to
`Console.Write(string)`, then the existing BASIC newline still flows through
`putchar`.

This is not full E4 backend coverage yet. CLR supports the literal-output shape
only; `str_len`, `str_index`, `str_concat`, and `str_eq` still fail closed until
the managed backend owns byte-string semantics. JVM/WASM/native/LLVM string
lowering remain follow-up slices.

