## 0.10.0 — 2026-06-23 — `entier` standard function (LANG-FULL E8 PR-7)

The ALGOL 60 standard function **`entier`** (§3.2.5) — the largest integer not
greater than a real (floor, toward −∞):

```algol
entier(2.7)   ⇒ 2
entier(-2.7)  ⇒ -3      ; NOT -2 — floor, not truncate-toward-zero
entier(42.0)  ⇒ 42
```

This is the first **frontend consumer** of the E8 numeric-conversion ops. Unlike
`abs`/`sign` (which synthesise a conditional), `entier` lowers to a **single**
`real_to_int_floor` IIR op — the floor and the real→integer narrowing fused into
the primitive — so each backend emits its native floor-then-convert
(`llvm.floor`+`fptosi`, `f64.floor`+`i64.trunc_sat`, `Math.floor`+`d2l`,
`Math::Floor`+`conv.ovf.i4`, `frintm`+`fcvtzs`, `roundsd`+`cvttsd2si`). The
floor-vs-truncate distinction is exactly why E8 provides a distinct
`real_to_int_floor` alongside `real_to_int_trunc`.

The operand must be `real` (`entier` is specifically the real→integer floor; an
`integer` argument is a type error). A user `integer procedure entier` still
overrides the builtin (`proc_sigs` is consulted first in `emit_call_common`).
Resolved like `abs`/`sign` — no grammar change (`entier(x)` already parses as a
`proc_call`). Eight frontend unit tests (floor, toward-−∞, exact-integer,
single-op lowering, real-required, arity, user-override, composition with `abs`),
executed on the VM. The 7-backend executed matrix proof lands alongside.

