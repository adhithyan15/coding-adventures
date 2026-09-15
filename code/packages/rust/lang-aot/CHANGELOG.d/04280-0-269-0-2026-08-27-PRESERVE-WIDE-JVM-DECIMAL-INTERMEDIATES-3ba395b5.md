## 0.269.0 - 2026-08-27 (preserve wide JVM decimal intermediates)

The JVM scalar concretization pass now keeps a module on its `i64`/Java `long`
model when an explicit `i64` constant cannot be represented by `i32`. This
prevents COBOL's scale-12 nested-division multiplier from being truncated and
makes `COMPUTE R = A / B + C` display `000533` on real Java, matching the other
LANG backends. A unit regression pins the module-wide width decision and the
existing LANG matrix cell provides the executed JVM proof.

