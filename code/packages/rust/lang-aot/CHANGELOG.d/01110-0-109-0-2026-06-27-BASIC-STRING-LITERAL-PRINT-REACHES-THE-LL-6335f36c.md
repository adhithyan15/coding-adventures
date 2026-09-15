## 0.109.0 — 2026-06-27 — BASIC string literal PRINT reaches the LLVM column (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **LLVM + WASM + JVM + CLR + VM + JIT**.
`iir-to-llvm` 0.17.0 emits each printable-ASCII literal as a private unmanaged
constant `{ i64 len, [len x i8] bytes }`, then lowers `print_str` to
`@__print_str(payload,len)`. The LLVM matrix runner compiles the same generic C
print runtime used for `__print_i64`, adding a `fwrite`-based `__print_str`.

This remains the literal-output shape only; `str_len`, `str_index`, `str_concat`,
and `str_eq` fail closed until the byte-string runtime lands. Native string
lowering remains the last backend column for this BASIC string-print row.

