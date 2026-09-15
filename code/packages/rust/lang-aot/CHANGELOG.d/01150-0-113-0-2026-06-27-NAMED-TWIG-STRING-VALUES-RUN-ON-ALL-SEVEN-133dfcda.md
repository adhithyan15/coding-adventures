## 0.113.0 — 2026-06-27 — Named Twig string values run on all seven backends (LANG-FULL E4)

The Twig matrix now executes three named string value programs on **native-AOT +
LLVM + WASM + JVM + CLR + VM + JIT**:

- `(define a "AB") (define b "CDE") (string-length (string-append a b))`
  returns exit code `5`.
- `(define s "HELLO") (if (string=? s "HELLO") 42 0)` returns exit code `42`.
- `(define s "ABC") (string-ref s 2)` returns exit code `67`.

`twig-ir-compiler` 0.27.0 keeps non-escaping top-level string defines in `main`
as typed `str_const` registers, so the existing E4 backend support can consume
named string values without dynamic `global_set`/`global_get` or `call_builtin`
paths. Reassignable string variables, captured strings, `let` string slots, and
the out-of-bounds `str_index` trap proof remain follow-up E4 slices.

