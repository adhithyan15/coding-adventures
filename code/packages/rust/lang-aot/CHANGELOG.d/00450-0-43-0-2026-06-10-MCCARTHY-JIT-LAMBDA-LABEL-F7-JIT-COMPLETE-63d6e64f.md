## 0.43.0 — 2026-06-10 — McCarthy **JIT lambda/LABEL** (F7) — **JIT COMPLETE; all eight backends done** (LANG77 / W15b)

`jit_lisp` registers `lispy_to_exit_code` (the polymorphic lambda-result exit
coercion — a runtime tag dispatch derived from `LispyValue`'s predicates, the only
builtin lambda needs beyond W15a's set). Together with the `vm-core` 0.3.0
register-sizing fix, McCarthy `LAMBDA`/`LABEL`/recursion now run on the universal
JIT. Verified by RUNNING (`tests/jit_mccarthy.rs`): `((LAMBDA (X) X) 5)`→5,
`((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1,
lambda-with-`COND`-body→100/200, recursive `LABEL`→7. **The JIT is the eighth and
final backend — McCarthy 1960 LISP now runs on every LANG VM backend (F1–F7).**

