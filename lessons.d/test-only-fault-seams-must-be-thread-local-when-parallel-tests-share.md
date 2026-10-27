---
category: Rust
---

# Test-only fault seams must be thread-local when parallel tests share a process

**What went wrong.** An ARM7TDMI gate-level rollback test armed a process-wide
atomic Boolean that forced the next DFF commit to fail. A new stable-Q
authority test ran in parallel in the same unit-test process, occasionally
consumed that one-shot failure first, and failed even though neither test was
wrong in isolation.

**Fix.** The deliberately injected commit failure is now a thread-local
one-shot `Cell<bool>`. Only the test thread that arms it can consume it, while
production behavior and rollback semantics stay unchanged.

**Do differently.** Test-only failure injection that is intended for one
operation must be scoped to that test's thread or instance. Do not use a
process-wide one-shot flag unless the entire suite serializes every possible
consumer.
