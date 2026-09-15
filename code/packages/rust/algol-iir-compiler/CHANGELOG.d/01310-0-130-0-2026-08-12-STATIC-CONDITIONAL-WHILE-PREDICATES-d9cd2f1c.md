## 0.130.0 — 2026-08-12 — static conditional while predicates

The initial `while` proof now evaluates a conditional boolean predicate when
its selector is statically known, inspecting only the selected branch. Dynamic
selectors and unknown selected predicates remain fail-closed.

