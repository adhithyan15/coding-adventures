## 0.194.0 — 2026-08-25 — parenthesized selector identities

Bounded static while analysis now treats parentheses as transparent around
exact scalar self-references and boolean, integer, or real neutral literals.
Parenthesized changing expressions remain conservative.

