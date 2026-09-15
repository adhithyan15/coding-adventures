## 0.283.0 - 2026-08-31 (Dartmouth BASIC mixed DATA parity)

The unified matrix now executes a Dartmouth BASIC program whose one DATA stream
interleaves numeric and string values. Numeric scalars and string-array elements
consume the first pass in source order; `RESTORE` rewinds the same pointer and a
second pass reads numeric and string scalars. NativeAOT, LLVM, WASM, JVM, CLR,
VM, and JIT must all print the same `42`, `OK`, `20`, `O` transcript.

The frontend represents the stream as parallel kind/numeric/string arrays and
checks the runtime kind before every typed load, avoiding an unportable tagged
value ABI while retaining deterministic mismatch traps on every backend.

