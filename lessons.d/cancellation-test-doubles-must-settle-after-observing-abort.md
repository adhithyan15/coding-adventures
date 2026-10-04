---
category: Testing & coverage
---

# Cancellation test doubles must settle after observing abort

A cancellation test reused a preparation promise that never resolved on its
second call. Production disposal correctly waited for host cleanup, but the
fake ignored its abort signal and made the test time out. Use a deferred only
for the operation whose cancellation is under test; later calls must either
settle normally or explicitly reject when their signal aborts.
