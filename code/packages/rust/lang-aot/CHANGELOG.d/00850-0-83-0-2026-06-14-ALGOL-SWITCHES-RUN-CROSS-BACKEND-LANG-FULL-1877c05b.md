## 0.83.0 — 2026-06-14 — ALGOL switches run cross-backend (LANG-FULL AL5)

`tests/lang_matrix.rs` gains an executed ALGOL **switch / computed goto**:

```algol
begin integer result; switch s := a1, a2, a3; integer i; i := 3;
  goto s[i];
  a1: result := 1; goto done;
  a2: result := 2; goto done;
  a3: result := 49; done:
end
```

⇒ exit **49**, running across native / LLVM / WASM / JVM / CLR / VM / JIT.
`algol-iir-compiler` (0.3.0) lowers `goto s[i]` to a 1-based `index == k ? jmp Lk`
chain over the portable `jmp`/`jmp_if_false`/`label` subset.

The switch's index test (`i == k`) is the **first ALGOL comparison ever run on a
code-gen backend**, and it surfaced a latent width bug: ALGOL emitted `cmp_*`
with a `bool` type_hint, so LLVM compared two `i64` operands at 1-bit `i1`
(`3 == 1` → `1 == 1` → true → wrong target) and emitted invalid IR `clang`
rejected outright — the matrix cell failed to run at all. Fixed in
`algol-iir-compiler` 0.3.0 by comparing at the **operand** width (`i64`), the
same fix the BASIC BA0 work applied. `s[3]` is the chosen index precisely
because an i1 compare mis-selects the first arm, so the cell proves the fix.

