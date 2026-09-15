## 0.220.64 - 2026-08-11 (ALGOL array value copies — seven-backend matrix)

One- and two-dimensional boolean-array matrix cells now prove true ALGOL
call-by-value isolation: each callee observes its own writes while the caller's
initialized cells remain unchanged on Native AOT, LLVM, WASM, JVM, CLR, VM,
and JIT. Nested forwarding tests use name-mode sibling formals where shared
mutation is intentional.

