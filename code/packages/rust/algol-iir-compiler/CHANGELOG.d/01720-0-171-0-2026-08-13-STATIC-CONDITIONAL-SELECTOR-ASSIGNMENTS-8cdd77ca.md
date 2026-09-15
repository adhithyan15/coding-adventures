## 0.171.0 — 2026-08-13 — static conditional selector assignments

A variable-free statically known conditional selector assignment now scans
only its selected leaf when deciding whether a known transitive selector stays
stable. Dynamic predicates, selected changing leaves, and unsupported effects
remain conservative without recursive effect inference.

