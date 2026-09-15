## 0.111.0 — 2026-06-27 — Twig literal string metadata runs on all seven backends (LANG-FULL E4)

The Twig matrix now executes `(string-length "HELLO")`,
`(string=? "HELLO" "HELLO")`, and
`(string-length (string-append "AB" "CDE"))` on **native-AOT + LLVM + WASM +
JVM + CLR + VM + JIT**, returning exit codes `5`, `1`, and `5` everywhere.

`twig-ir-compiler` 0.25.0 lowers literal `string-length`, `string=?`, and
`string-append`-feeding-`string-length` to typed `str_const` +
`str_len`/`str_eq`/`str_concat`. The code-gen backends each keep the slice
literal-only: native AOT folds to constants, LLVM/WASM read literal metadata,
JVM calls `String.length()` / `String.equals(Object)` / `String.concat(String)`,
and CLR calls `String::get_Length()` / `String::Equals(string,string)` /
`String::Concat(string,string)`.

This extends the all-backend E4 foothold beyond output without claiming full
string algebra: `str_index`, non-literal `str_concat`/`str_eq`, and string
variables remain follow-up work.

