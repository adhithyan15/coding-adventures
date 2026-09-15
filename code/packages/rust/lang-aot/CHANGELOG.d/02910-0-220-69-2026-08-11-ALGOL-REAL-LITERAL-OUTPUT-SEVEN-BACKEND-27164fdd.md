## 0.220.69 - 2026-08-11 (ALGOL real-literal output — seven-backend matrix)

An ALGOL `output(4.25)` matrix cell now prints `4.25` on Native AOT, LLVM,
WASM, JVM, CLR, VM, and JIT. The compiler preserves the direct literal's source
spelling and reuses `str_const` plus `print_str`; runtime real formatting remains
explicitly unsupported rather than introducing a partial `f64` formatter ABI.

