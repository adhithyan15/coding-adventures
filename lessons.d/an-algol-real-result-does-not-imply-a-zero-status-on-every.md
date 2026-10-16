---
category: Compiler / VM / language pipeline
---

# An ALGOL real result does not imply a zero status on every backend

The cap23 runtime spec incorrectly demanded Some(0) for unchanged sources with
a top-level real result. Compiler::finish deliberately returns that f64. Actual
e5df first executed16 saved-variable cases on6 backends (96 Exit0 tuples), then
result-context17 on NativeAot/LLVM before Wasm reported completed code21/stdout21
and the universal-zero assertion failed. This is a real contract failure, not
a demonstrated runtime trap. Preserve the red and exact source bytes.

Trace each backend's typed entry return and process adapter before specifying
status. JVM Stdout discards it; WASM converts F64 numerically; VM/JIT as_i64 can
return None for Float; native/LLVM need explicit ABI scope. Require exact
reviewed status and typed-result observation, never arbitrary code or stdout-only
acceptance, and keep existing saved-only normalExit0 controls unchanged.
