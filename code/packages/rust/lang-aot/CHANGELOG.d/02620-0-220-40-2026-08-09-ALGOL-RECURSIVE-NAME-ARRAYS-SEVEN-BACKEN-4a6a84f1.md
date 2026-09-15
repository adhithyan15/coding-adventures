## 0.220.40 - 2026-08-09 (ALGOL recursive name arrays — seven-backend matrix)

The matrix now runs a mutually recursive ALGOL name-array pair on Native AOT,
LLVM, WASM, JVM, CLR, VM, and JIT. The two descriptor-backed siblings write
through the same three caller cells before the base case sums them, proving
that recursive direct calls retain the array handle and bounds.

