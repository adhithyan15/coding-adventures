## 0.165.0 — 2026-08-13 — transitive idempotent assignment dependencies

Checked static idempotent assignments may now depend on an ordinary local
scalar that is itself only exactly self-assigned in the loop body. Computed or
cross-assigned dependency writes, loop controls, and unsupported bindings still
fail closed without recursive effect inference.

