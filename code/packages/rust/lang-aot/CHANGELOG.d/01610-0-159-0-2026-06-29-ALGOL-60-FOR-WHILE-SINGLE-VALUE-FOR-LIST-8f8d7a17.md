## 0.159.0 — 2026-06-29 — ALGOL 60 for-while + single-value + for-list (LANG-FULL AL11)

Three new matrix `Prog` cells proved on all 7 backends (NativeAot, Llvm, Wasm,
Jvm, Clr, Vm, Jit):

- **`for … while`**: `for i := i + 6 while i <= 36 do result := i + 6` — starts
  at 0, advances by 6 each iteration until i > 36; the body captures the
  stepped value; last assigned result = 42 on iteration where i=36.
  The `emit_for_while` IIR lowering emits a label → expression code →
  `jmp_if_false` → body → `jmp` skeleton (same as `step-until`).

- **Single-value for element**: `for i := 2 do result := 40 + i` — executes
  the body exactly once with i=2 → result=42.  The `emit_for_value` path
  emits a single `mov` + body block with no loop at all.

- **Multi-element for list**: `for i := 1 step 1 until 3, 10, i + 1 while i < 13`
  sequences three for-element kinds in one head: `step-until` (i=1,2,3),
  a literal value (i=10), and `while` (i=11,12).  Sum 1+2+3+10+11+12=39,
  then `result := result + 3` → 42.  Multi-element lowering chains the
  exit-of-one / init-of-next blocks sequentially.

852 total proven cells pass.

