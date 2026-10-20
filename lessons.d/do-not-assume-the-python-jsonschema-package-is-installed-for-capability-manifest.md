---
category: Testing & coverage
---

# Do not assume the Python jsonschema package is installed for capability-manifest checks

A fresh repository worktree had Python available but not the third-party
`jsonschema` module, so an ad hoc capability-manifest validation snippet failed
before checking any data. Required-capability validation must use a
repository-owned checker or the build tool's existing schema gate, not an
undeclared local Python package. If no focused checker exists, inspect the
reviewed schema and run the normal build-tool validation path rather than
silently installing a new dependency.
