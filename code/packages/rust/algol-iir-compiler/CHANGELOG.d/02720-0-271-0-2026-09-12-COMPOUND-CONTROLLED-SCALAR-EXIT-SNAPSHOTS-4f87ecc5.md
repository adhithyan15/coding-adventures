## 0.271.0 — 2026-09-12 — compound controlled-scalar exit snapshots

Single-iteration `step`/`until` analysis now recognizes an exact controlled
scalar assignment wrapped in a one-statement `begin`/`end` body and retains
the checked post-body increment when it exits the loop. Multi-statement bodies
continue to fail closed.

