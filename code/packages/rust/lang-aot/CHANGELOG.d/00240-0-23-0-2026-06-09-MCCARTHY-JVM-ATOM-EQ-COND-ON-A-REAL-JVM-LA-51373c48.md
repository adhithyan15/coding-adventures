## 0.23.0 — 2026-06-09 — McCarthy → **JVM `ATOM`/`EQ`/`COND`** on a real JVM (LANG77 / W4)

The JVM backend now lowers the lisp predicates (`pair?`/`not`/`equal?`), so
McCarthy `ATOM`, `EQ`, and `COND` run on a real `java`: `(ATOM 5)`→1,
`(ATOM (CONS 1 2))`→0, `(EQ 5 5)`→1, `(EQ 5 6)`→0, `(COND ((EQ 1 1) 7) (5 9))`→7.
Same shared structural pass as wasm — only the per-builtin JVM lowering is new
(`instanceof Object[]` / `ixor` / `checkcast`+`intValue`+`if_icmpeq`). The
real-`java` test harness (`tests/jvm_predicates.rs`) is now **descriptor-aware**:
a predicate result is `int` (`()I`), a COND selecting an integer atom is `long`
(`()J`) — it picks the matching `println` overload. (Symbols — F6 — are W5: their
interned ids land in a high range that needs `ldc`, handled separately.)

