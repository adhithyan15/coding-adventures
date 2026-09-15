## 0.220.68 - 2026-08-11 (ALGOL boolean output — seven-backend matrix)

An ALGOL `output(flag and true, flag and false)` matrix cell now prints
`truefalse` on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. The frontend
branches on each typed boolean and reuses the existing `str_const` and
`print_str` substrate on both paths.

