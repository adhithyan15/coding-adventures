## 0.268.0 — 2026-09-12 — static boolean negation snapshots

Static boolean evaluation now recognizes only exact bare variables before
examining operators, so unary `not` wrappers are evaluated instead of being
mistaken for identity reads. Bounded while-loop boolean recurrences can
therefore retain exact negated snapshots.

