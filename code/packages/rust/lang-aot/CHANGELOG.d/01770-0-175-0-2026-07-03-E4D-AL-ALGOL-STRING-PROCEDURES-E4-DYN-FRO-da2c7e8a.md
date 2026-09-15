## 0.175.0 — 2026-07-03 — E4d-AL: ALGOL string procedures (E4-dyn frontend payoff)

Added a matrix cell for the first E4-dyn *frontend* payoff — ALGOL 60
`string procedure`s (algol-iir-compiler 0.28.0):

```algol
begin string procedure pick(n); value n; integer n;
    if n > 0 then pick := 'HI' else pick := 'LO';
print(pick(1)) end
```

This proves the full chain — string-procedure declaration + call + runtime-string
**return value** + print — end-to-end on the backends that already carry a runtime
string arriving as a *call result*: **NativeAot** and **VM/JIT**. (The E4-dyn
foothold only ever printed a branch-selected *local*; a string return value is a
new path.) The LLVM/WASM/JVM/CLR columns take their runtime-string path only for
promoted-slot operands, so a string return value on those backends is the E4d-2b
(LLVM) / E4d-3b (WASM) / JVM+CLR follow-up that will extend this cell to all seven.

Bringing this up surfaced and fixed a latent native miscompile (twig-aot 0.28.0):
`strip_dead_aot_string_allocs` dropped all but the last buffer of a multi-block
string alias, so a not-last branch printed `""`. Both matrix guard tests pass.

