## 0.164.0 — 2026-07-01 — LANG-STR-RT: 7 NativeAot string-parameter matrix cells unlocked

Seven `lang_matrix.rs` test cells that were previously excluded from `NativeAot`
(with `// NativeAot: str_len/str_eq on function params not yet wired`) now pass
on NativeAot.  The change adds `NativeAot` to the `backends` array for each of
these `Prog` entries:

- `(define (strlen (s : str)) (string-length s)) (strlen "HELLO")`
- `(define (print_hi (s : str)) (print_str s)) (print_hi "HELLO\n")`
- `(define (same? (a : str) (b : str)) (string=? a b)) (same? "X" "X")`
- `(define (diff? (a : str) (b : str)) (string=? a b)) (diff? "A" "B")`
- `(define (concat_len (a : str) (b : str)) ...) (concat_len "HE" "LLO")`
- `(define (greet (name : str)) ...) (greet "World")`
- `(define (initial (s : str)) (string-ref s 0)) (initial "ABC")`

The unlock is enabled by the LANG-STR-RT buffer layout (8-byte length prefix
at offset 0, data at offset 8+), implemented in `twig-aot` 0.24.0, combined
with the `str_eq` V1 builtin in `aarch64-backend` 0.18.0 / `x86_64-backend`
0.20.0.

