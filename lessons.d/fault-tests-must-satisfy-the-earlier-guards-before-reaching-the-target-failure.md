---
category: Testing & coverage
---

# Fault tests must satisfy the earlier guards before reaching the target failure

A checksum-overflow test intended to reach the timed-batch overflow guard, but
its candidate returned a different one-operation warmup checksum first. After
fixing that, the default calibration target was already met at batch size one,
so collection never attempted the overflowing size. Keep each earlier
precondition valid, set the target so calibration reaches the intended batch,
and make the callback fail only at the boundary under test; then assert the
exception type or message for that boundary.
