# twig-jvm-compiler

Twig → JVM class file (`*.class`) compiler — the first non-Python
target for Twig, running on **real `java`**, not a simulator.

See [TW02 spec](../../../specs/TW02-twig-jvm-compiler.md) for the
v1 surface and the multi-step roadmap.

## Architecture

```
Twig source
   ↓  parse_twig + extract_program        (twig package)
typed AST
   ↓  twig_jvm_compiler.compile_to_ir     (this package)
IrProgram
   ↓  ir-optimizer
IrProgram (optimised)
   ↓  lower_ir_to_jvm_class_file          (existing)
.class file bytes
   ↓  java -cp <dir> <ClassName>          (real JVM)
program output
```

## V1 surface

- Integer literals, booleans
- Arithmetic: `+`, `-`, `*`, `/`
- Comparison: `=`, `<`, `>`
- Control flow: `if`, `let`, `begin`
- **Top-level `define`** for both values (literal RHS) and
  functions
- Recursive function calls
- Output: the program's final value is written as a single byte
  to stdout via `SYSCALL 1`

## Quick start

```python
from twig_jvm_compiler import compile_source, run_source

# Compile to .class bytes:
result = compile_source("(+ 1 2)")
print(len(result.class_bytes), "bytes")

# Compile + run on real java:
out = run_source("(+ 1 2)")
print(out.stdout)  # b'\x03'
```

## Testing on real `java`

The integration tests in `tests/test_real_jvm.py` invoke
`subprocess.run(["java", ...])` and assert on actual JVM output.
They skip cleanly when `java` is not on PATH.
