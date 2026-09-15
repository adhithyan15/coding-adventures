## 0.172.0 — 2026-08-13 — stable conditional selector assignments

A statically known predicate over unchanged ordinary local boolean, integer,
or real scalars may now select the preserving leaf of a conditional assignment
to a known transitive selector. Written, controlled, nonlocal, array, by-name,
and unsupported predicate dependencies remain conservative without recursive
effect inference.

