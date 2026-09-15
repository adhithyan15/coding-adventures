## 0.223.0 — 2026-08-27 — conditional integer subexpressions

Exact integer snapshot evaluation now accepts conditional subexpressions when
their selected branch is statically known or both independently proven branch
values agree. Runtime branching and path-dependent values remain dynamic.

