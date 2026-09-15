## 0.175.0 — 2026-08-14 — stable nested selector dependencies

A statically known predicate over unchanged ordinary local scalars may now
select the preserving leaf of a conditional assignment to a known predicate
dependency that itself selects a preserving conditional selector assignment.
The proof is capped at one nested dependency level; cycles, deeper chains,
written selectors, and unsupported effects remain conservative.

