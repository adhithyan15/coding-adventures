## 0.220.80 - 2026-08-11 (ALGOL static real standard functions — seven backends)

The LANG matrix now proves formatter-free `abs` and exact `sqrt` output over
literal-only real arithmetic on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.
The frontend still defers inexact roots, invalid domains, runtime values, and
user-overridden standard-function names to the unsupported runtime formatter.

