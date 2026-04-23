# JVM Runtime

Top-level orchestration for the modular JVM prototype.

This package composes:
- `jvm-class-file`
- `jvm-bytecode-disassembler`
- `jvm-simulator`

For the current prototype it includes a tiny host bridge for
`System.out.println(String)`, which is enough to run a real compiled Java
hello-world class end to end.
