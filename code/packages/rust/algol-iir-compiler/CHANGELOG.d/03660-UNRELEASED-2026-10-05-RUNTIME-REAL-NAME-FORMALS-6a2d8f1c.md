## 0.366.0 - 2026-10-05 - Runtime-real read-only name formals

Specialised real call-by-name formals now retain runtime-real formatter
provenance when their actual is a non-assignable expression already covered by
the bounded proof. Direct output and composition through the formal therefore
reuse the portable real formatter without adding a runtime thunk ABI.

Assignable actuals remain conservative because a write through the name formal
could invalidate immutable provenance.
