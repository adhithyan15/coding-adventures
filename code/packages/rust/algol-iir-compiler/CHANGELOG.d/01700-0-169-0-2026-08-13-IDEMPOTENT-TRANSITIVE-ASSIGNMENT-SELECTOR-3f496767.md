## 0.169.0 — 2026-08-13 — idempotent transitive assignment selectors

An exact bare self-assignment may now leave a known ordinary local selector
stable while it chooses the preserving leaf of a transitive assignment
dependency. Computed assignments, loop controls, and unsupported selector
effects remain conservative without recursive effect inference.

