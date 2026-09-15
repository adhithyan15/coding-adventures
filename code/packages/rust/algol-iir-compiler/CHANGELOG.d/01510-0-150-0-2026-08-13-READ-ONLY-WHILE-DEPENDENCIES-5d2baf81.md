## 0.150.0 — 2026-08-13 — read-only while dependencies

Stable local dependencies used by capped `while` analysis may now be read by
the loop body. An AST write scan still rejects assignment targets and nested
controlled variables; calls and existing control-flow barriers continue to
invalidate static tracking.

