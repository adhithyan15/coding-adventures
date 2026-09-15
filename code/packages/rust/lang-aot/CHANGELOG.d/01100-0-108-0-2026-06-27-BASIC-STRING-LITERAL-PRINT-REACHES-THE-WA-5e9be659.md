## 0.108.0 — 2026-06-27 — BASIC string literal PRINT reaches the WASM column (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **WASM + JVM + CLR + VM + JIT**.
This extends the managed/static literal-output foothold to the in-repo WASM runtime:
`iir-to-wasm` stores printable ASCII string literals in a linear-memory data segment,
materialises the `str` value as an `i32` pointer, and lowers `print_str` to the host
import `env.__print_str(ptr,len)`. The existing BASIC newline still flows through
`putchar`.

This remains the literal-output shape only; `str_len`, `str_index`, `str_concat`,
and `str_eq` fail closed until the byte-string runtime lands. Native and LLVM
string lowering remain follow-up slices.

