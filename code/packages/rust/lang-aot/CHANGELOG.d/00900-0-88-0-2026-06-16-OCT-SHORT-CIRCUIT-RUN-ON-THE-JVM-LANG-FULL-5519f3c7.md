## 0.88.0 — 2026-06-16 — Oct `&&`/`||` short-circuit run on the JVM (LANG-FULL, BA-JVM-1 follow-through)

The two Oct short-circuit matrix programs (`&&` → `9`, `||` → `7`) now include the
**JVM** backend: `iir-to-jvm-class-file` 0.13.3 makes a `mov` bridge int↔long when
the dest slot width differs (Oct keeps the i64 value model, so a bool comparison
result mov'd into a long accumulator needs `i2l`). With this, **every matrix
program runs on all 7 backends**.

