## 0.173.0 — 2026-08-13 — idempotent conditional selector dependencies

An exact bare self-assignment may now leave a known ordinary local predicate
dependency stable while that predicate selects the preserving leaf of a
conditional assignment to a known transitive selector. Computed assignments,
loop controls, and unsupported effects remain conservative without recursive
effect inference.

