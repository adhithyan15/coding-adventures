## 0.112.0 — 2026-06-27 — Twig literal string index runs on all seven backends (LANG-FULL E4)

The Twig matrix now executes `(string-ref "ABC" 1)` on **native-AOT + LLVM +
WASM + JVM + CLR + VM + JIT**, returning exit code `66` everywhere.

`twig-ir-compiler` 0.26.0 lowers direct-literal `string-ref` to typed
`str_const` + integer `const` + `str_index`. Native AOT and LLVM fold from
literal metadata, WASM emits a guarded `i32.load8_u` from its string data
segment, JVM calls `String.charAt(I)`, and CLR calls
`String::get_Chars(int32)`. This extends the all-backend E4 foothold to
in-bounds ASCII indexing without claiming dynamic string variables or the
out-of-bounds trap matrix proof yet.

