## 0.198.0 — 2026-08-25 — transitive identity dependencies

Bounded static while analysis now recognizes complete supported integer and
boolean identity writes while checking a transitive value or predicate
dependency. Changing writes remain conservative, and the check does not add
recursive effect inference.

