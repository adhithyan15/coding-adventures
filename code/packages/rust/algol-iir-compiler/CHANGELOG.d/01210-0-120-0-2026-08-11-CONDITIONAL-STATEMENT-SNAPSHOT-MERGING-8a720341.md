## 0.120.0 — 2026-08-11 — conditional-statement snapshot merging

Statement conditionals now compile each branch from the same post-condition
metadata state and retain only identical integer and real snapshots at the
join. Missing `else` branches merge against the unmodified path, while calls
and every existing invalidation boundary continue to poison the result.

