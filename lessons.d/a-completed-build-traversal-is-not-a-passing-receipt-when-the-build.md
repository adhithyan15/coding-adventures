---
category: CI & GitHub Actions
---

# A completed build traversal is not a passing receipt when the build tool crashes before its summary

On PR #17185 at source head 977b3bfa79f332c2a0a9dadc5d99bccf5628450b,
macOS job 114812563161 reached `5325/5325 done` after 5512.4 seconds at
2026-10-12 15:45:34 UTC. The Go 1.26.1 darwin/arm64 build tool then reported
`unexpected fault address`, `SIGBUS` and `panic during panic`, before the
result table and failure exit decision were available. The runner stayed open
until cancellation and reported `Illegal byte sequence` while handling its step
summary and diagnostic log. The earlier live log API returned only the initial
portion of the log, which did not expose this crash until the job completed.

The package traversal counts include all result states. They do not establish
that 535 packages passed, that coverage succeeded, or that the aggregate CI gate
is acceptable. The Windows and Linux result tables for that same source head
were independently complete; they do not substitute for the missing macOS
receipt.

Recovery is a fresh hosted-runner attempt with the reviewed source and the same
assertions, workers, heap, coverage and deadlines. Keep the failed log and require
a terminal successful job, its zero-failure result table and the protected CI
gate before declaring recovery. The exact cause of this one SIGBUS is not
established; a later successful attempt is recovery evidence, not proof that the
original fault was transient or caused by a particular service incident.

Do not change production code or broaden resource limits solely from a native
runtime crash with an incomplete traceback. If it repeats, compare the exact
phase and retained diagnostics, then investigate executable identity, toolchain,
runner and storage behavior before proposing a focused repair.
