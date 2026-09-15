## 0.110.0 — 2026-06-27 — BASIC string literal PRINT runs on all seven backends (LANG-FULL E4 / BA4)

The Dartmouth BASIC `PRINT "HELLO"` matrix row now runs on **native-AOT + LLVM + WASM + JVM + CLR + VM + JIT**.
`twig-aot` 0.15.0 lowers native `str_const` + `print_str` to the existing heap-byte
runtime path (`alloc_bytes`, `store_byte`, `call_builtin "print_string"`), so the
native executable links `__twig_print_string` from the existing AOT runtime archive.

This completes the all-backend literal-output proof. Richer byte-string ops
(`str_len`, `str_index`, `str_concat`, `str_eq`) and string variables remain
follow-up E4/BA4 work.

