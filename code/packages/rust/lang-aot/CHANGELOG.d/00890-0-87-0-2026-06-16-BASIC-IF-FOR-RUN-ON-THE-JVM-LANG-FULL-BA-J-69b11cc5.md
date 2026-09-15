## 0.87.0 — 2026-06-16 — BASIC `IF`/`FOR` run on the JVM (LANG-FULL BA-JVM-1)

The two Dartmouth BASIC control-flow matrix programs (the `FOR` sum → `15` and
the `IF` branch → `7`) now include the **JVM** backend: `iir-to-jvm-class-file`
0.13.2 fixes the comparison-dest slot typing that made a branch-after-a-loop over
BASIC's i64 value model fail JVM verification (`uninitialized register pair`).
Both run on real `java` now; the matrix proves it cross-backend.

