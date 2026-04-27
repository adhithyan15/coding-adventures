# nib-jvm-compiler

## 0.1.0 - 2026-04-18

- Add the initial Python Nib-to-JVM pipeline package.
- Orchestrate Nib parsing, type checking, IR lowering, optional IR optimization,
  JVM class-file lowering, and class-file parsing.
- Add helpers for writing generated classes into a classpath root.
- Add parseable class-file tests, stage-labeled error tests, and optional
  GraalVM execution smoke tests.
