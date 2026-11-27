---
category: CI & GitHub Actions
---

# An unavailable getdents64 result needs same-generation exit evidence before a dying-thread test accepts it

The required real-strace `TestSandboxStraceDyingThreads` check failed on
`getdents64(9</tmp/.../pkg>) = ? <unavailable>`, refused as
`unresumed-call`. The production trace parser correctly refuses directory
reads whose result is missing. The test log named the refused PID, but printed
only the trace tail after it, so the historical run did not establish whether
that PID later terminated.

If this exact spelling appears again, keep production parsing fail-closed. A
test-only exception must require the real trace to contain a terminal record
that the production parser accepts for the same PID generation, and must reject
missing or malformed exits, exits for another PID, and a numeric PID reused by
a later spawn. Do not infer that `getdents64` was harmless from the failed
assertion alone. Print a bounded sequence of later events for the refused PID
and any spawn evidence so the next failure exposes whether a terminal event
belongs to that generation.
