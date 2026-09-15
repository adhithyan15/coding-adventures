## 0.220.41 - 2026-08-09 (ALGOL recursive scalar name forwarding — seven-backend matrix)

The matrix now runs a mutually recursive ALGOL scalar name-forwarding pair on
Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT. Three recursive write-through
increments carry the original caller variable from 21 to 42 before the base
case reads it through the preserved name binding.

