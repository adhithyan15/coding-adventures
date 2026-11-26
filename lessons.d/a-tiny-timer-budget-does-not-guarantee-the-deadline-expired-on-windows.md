---
category: Testing & coverage
---

# A tiny timer budget does not guarantee the deadline expired on Windows

`TestVerifyTreeBudget` used a one-nanosecond budget and assumed the timer would
expire before verification checked its context. On Windows, timer resolution
and scheduling can let the work finish first, making the test flaky without a
product bug. Use zero or a negative budget when the assertion is specifically
about an already-expired context; Go expires non-positive deadlines
synchronously.

