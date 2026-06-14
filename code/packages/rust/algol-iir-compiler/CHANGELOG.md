# Changelog

## 0.2.0 — ALGOL 60 typed procedures with value parameters (LANG-FULL AL3)

- Lower **typed (function) procedures with `value` parameters** to sibling
  `IIRFunction`s in the module. A heading like
  `integer procedure sq(x); value x; integer x; sq := x*x` becomes a function
  `sq(x: i64) -> i64`, and a call `sq(7)` (in expression or statement position)
  becomes an IIR `call` whose `srcs[0]` names the callee. Procedure signatures
  are registered in a pre-pass over each block, so a procedure may be called
  before it is textually declared and may call itself (recursion).
- Proven by **running**: `tests/lang_matrix.rs` in `lang-aot` executes
  `result := sq(7)` ⇒ exit `49` across native-AOT / LLVM / WASM / JVM / CLR /
  VM / JIT. Unit tests cover multi-parameter procedures, boolean procedures,
  recursion (factorial via an if-statement body), statement-position calls, and
  the rejection paths (void procedures, call-by-name parameters, arity and type
  mismatches).
- **Scope and limitations** (tracked as follow-ups): only typed procedures with
  `value` parameters are modelled. Proper (void) procedures are rejected — they
  have no observable effect on the current executable slice (no output op, no
  by-reference or enclosing-scope mutation), so admitting one would lower code
  no test could witness. Procedure bodies are lexically flat: they see their
  own value parameters but not enclosing-block variables (call-by-name /
  Jensen's device and non-local access are future work).

## 0.1.0

- Add an ALGOL 60 scalar frontend for the LANG VM Rust chain.
- Lower integer and boolean declarations, scalar assignments, integer arithmetic including `div`/`mod`, comparisons, if/else, compound statements, goto labels, and simple `for step until` loops to `interpreter_ir::IIRModule`.
- Prove the emitted IIR runs through `vm-core`, `jit-core`, `aot-core`, WebAssembly, JVM, CLR, BEAM, and LLVM backend paths.
