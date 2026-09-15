## 0.21.0 — 2026-06-29 — Real procedure unit test (LANG-FULL AL13-real-proc)

Added `real_procedure_runs` unit test covering the `real procedure` code path that
was already implemented but untested:

- `scale(x) = x * 6.0` (real procedure, single real parameter)
- Caller: `result := entier(scale(7.0))` — asserts 42 (42.0 floored)

This exercises `ScalarType::Real` as a procedure return type end-to-end through
the VM: `declare_var` seeds the `scale` slot as f64, the body assigns to it, `ret`
returns the f64, and the caller's `emit_entier` floors it to i64.

No compiler code changed — this is a coverage gap closure only.

