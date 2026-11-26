---
category: Testing & coverage
---

# A tiny timer budget does not guarantee the deadline expired on Windows

`TestVerifyTreeBudget` set a one-nanosecond budget and assumed the timer would
expire before verification checked its context. Windows timer granularity can
let the work finish first, making the test flaky without indicating a product
bug. Use a zero or negative budget when the assertion is specifically about an
already-expired context; Go returns `context.DeadlineExceeded` synchronously
for a non-positive deadline.
