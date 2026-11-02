---
category: Testing & coverage
---

# A typed allocation error needs a deterministic executable boundary

**What went wrong.** A transactional simulator exposed `AllocationFailed` and
used fallible reservation for a few containers, but ordinary mnemonic strings
still used infallible conversions. The advertised error therefore could not
cover every trace-construction allocation, and ordinary process OOM behavior
was not a testable proof that a partially executed instruction or run rolled
back.

**Fix.** Route each fallible output allocation through a named preflight:
mnemonic text, register-effect vectors, special-effect vectors, complete trace
construction, retained-run traces, and truncated-fetch evidence. Add inert
production seams with deterministic test-only failure injection. Force every
site and also fail a later step after an earlier step succeeds, then compare
the full machine state with the `step`, `run`, or `execute` call-entry snapshot.

**Next time.** When an API promises a typed resource failure, enumerate every
allocation on the success path before implementation review. A fallible
reservation in one container is not enough; every later allocation must be
preflighted, injectable, and covered by exact-state rollback tests.
