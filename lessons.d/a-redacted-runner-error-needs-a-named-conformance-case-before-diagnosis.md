---
category: CI & GitHub Actions
---

# A redacted runner error needs a named conformance case before diagnosis

CI run 38190146667 built 534 Linux packages but failed Python's cross-language
plugin-runner conformance entrypoint with RPC -32603, INTERNAL_ERROR. All 22
Python unit tests passed, and the same package built on Windows. The harness's
outer message named the whole canonical corpus, not its failing request or case.

Keep production error redaction intact. Temporarily instrument the trusted
conformance driver's pending request rejection, recording the method and fixture
operation, and repeat the real coverage-wrapped subprocess entrypoint. Do not
infer a backend bug or change deadlines from a generic redacted error alone.
The local Python 3.12 direct invocation and Python 3.13 coverage-wrapped invocation
passed. Ten consecutive coverage-wrapped canonical conformance runs also passed on the local Windows host with Python 3.13.14. The Linux error remains unreproduced; the next actual Linux CI result is still required.

