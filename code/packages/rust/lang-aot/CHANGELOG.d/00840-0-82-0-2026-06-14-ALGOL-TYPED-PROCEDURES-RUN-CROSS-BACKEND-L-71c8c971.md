## 0.82.0 — 2026-06-14 — ALGOL typed procedures run cross-backend (LANG-FULL AL3)

`tests/lang_matrix.rs` gains the first **multi-function ALGOL** program: a typed
procedure with a value parameter,

```algol
begin integer result;
  integer procedure sq(x); value x; integer x; sq := x * x;
  result := sq(7)
end
```

⇒ exit **49**, running across native / LLVM / WASM / JVM / CLR / VM / JIT. The
`algol-iir-compiler` (0.2.0) lowers the procedure to a sibling
`IIRFunction sq(x: i64) -> i64` and the call to an IIR `call`; every backend
already resolves a same-module `call` by name, so procedures are "just functions
+ calls" in the shared IIR — no backend learned anything about ALGOL.

Running this across all backends **surfaced a real `jit-core` bug**: the
`CIROptimizer`'s constant propagation never invalidated a register's known
constant when the register was reassigned, so a procedure's result slot
(`const sq=0; …; mov sq=t; ret sq`) propagated the dead `0` and the JIT returned
`0` instead of `49` while every other backend agreed on `49`. Fixed in `jit-core`
0.4.1 (reassignment-kills + block-boundary-clears). The executed matrix is
exactly the guardrail that caught it.

