---
category: Repo policy / workflow reminders
---

# Consume a producer exit before invoking a helper that it creates

A preparation command returned an active session before writing its dependent materializer. The next command was incorrectly invoked immediately and Python refused the missing helper with exit1. No source or index was written, and no incoming source executed.

An awaited exec call can still return a running session. Inspect that result, consume the actual producer exit, and only then invoke a helper or use an artifact it creates. Keep dependent operations sequential; parallelize only independent reads. The producer subsequently completed0, and the materializer was invoked separately.
