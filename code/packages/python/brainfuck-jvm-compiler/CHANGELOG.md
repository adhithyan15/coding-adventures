# brainfuck-jvm-compiler

## 0.1.0 - 2026-04-18

- Add the initial Python Brainfuck-to-JVM pipeline package.
- Orchestrate Brainfuck parsing, IR lowering, optional IR optimization, JVM
  class-file lowering, and class-file parsing.
- Add helpers for writing generated classes into a classpath root.
- Add parseable class-file tests and optional GraalVM execution smoke tests.
