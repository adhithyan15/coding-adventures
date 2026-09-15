## 0.9.0 — 2026-06-22 — `sign` standard function (LANG-FULL AL8, PR-2)

The second ALGOL 60 standard function (§3.2.4), **`sign`**, building on the
`abs` machinery from 0.8.0.

- `sign(E)` is the *signum*: `+1` if `E > 0`, `-1` if `E < 0`, `0` if `E = 0`.
  Unlike `abs`, the **result is always `integer`** regardless of the operand's
  type — `sign(-2.5)` is the integer `-1` (no real→integer coercion needed at
  the use site).  The operand may be `integer` or `real`.
- It lowers to the nested conditional `if E > 0 then 1 else if E < 0 then -1
  else 0`: a `cmp_gt` then a `cmp_lt` against a typed zero (compared at the
  operand width), with three `i64` constants moved into one result slot — the
  same store-per-branch shape (no SSA phi) `abs` uses, so it **runs on all seven
  backends** (native-AOT/LLVM/WASM/JVM/CLR/VM/JIT).  `E` is evaluated once.
- Same name-based, overridable resolution as `abs`: a user `procedure sign`
  wins over the built-in.
- **Verified by RUNNING:** a `lang_matrix.rs` cell — `43 + sign(0 - 1)` ⇒ exit
  **42** (the negative branch) — executes on every backend; plus 8 inline tests
  (positive / negative / zero integer `sign`, positive / negative real `sign`
  yielding an integer, composition with `abs`, the user-override case, and the
  wrong-arity rejection).

`entier` (floor of a real → integer) needs a float-floor+convert that is not a
portable IIR op, and `sqrt`/`sin`/`cos`/… need a runtime math library on every
backend; those are later AL8 slices.

