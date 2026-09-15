## 0.174.0 — 2026-08-13 — conditional idempotent selector dependencies

A conditional assignment whose leaves are all the same bare scalar may now
leave a known predicate dependency stable while that predicate selects the
preserving leaf of an assignment to a known transitive selector. Differing,
computed, controlled, and unsupported effects remain conservative without
recursive effect inference.

