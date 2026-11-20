---
category: CI & GitHub Actions
---

# A REQUIRED real-strace test that tolerates a closed list of refusals flakes on the next kernel-timing shape

**What went wrong.** `TestSandboxStraceDyingThreads` is a REQUIRED kiln-sandbox
test from the dying-call rule (D35, #19735). It kills Go workers mid-syscall and
tolerates only refusals in a closed list (`unresumed-call`, `unparsable-line`) on
known dying shapes. A later unrelated PR (#19750) hit a new shape on
ubuntu-24.04: a worker killed between its `openat`'s kernel return and strace's
exit stop resumed with `= -1 (errno 18446744073709551544)` (-72 read as
unsigned), then exited. Both readers refuse that as `result`. This fails closed and is
correct, but it was not on the test's list, so the required job failed for a PR that
had nothing to do with tracing.

**Fix.** The test now also tolerates `result`, but only on a resumed line
whose result is `-1 (errno N)` and whose pid's very next line is its own exit.
Every other refusal still fails.

**Do differently.** A required test driven by kernel timing must tolerate every
fail-closed refusal that timing can produce, and fail only on what would be
unsound: a cross-check mismatch, an allowed divergence, or an acceptance the policy
forbids. Asserting on the exact refusal reason turns each new timing shape into a
repo-wide flake. When adding such a test, list the refusals by their safety
property, not by their spelling.
