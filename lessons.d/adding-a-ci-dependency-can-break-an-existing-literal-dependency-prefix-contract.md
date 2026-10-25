---
category: CI & GitHub Actions
---

# Adding a CI dependency can break an existing literal dependency-prefix contract

The NN37 required-conformance change inserted its job after `contracts` in the
required roll-up's dependency list. Neural conformance and CodeQL passed, but
the existing OCaml metadata test rejected the changed literal prefix
`needs: [detect, contracts, ocaml-capability-gate,`.

Append the new dependency instead. This preserves the existing source contract
without weakening either OCaml validation or the new selected-conformance result
check. Dependency-list order has no scheduling meaning, but a source-level
regression test may still pin it.

When editing a shared workflow, search repository tests for the exact changed
fragments and run those suites alongside the new gate tests. Package-local
tests and a registry consistency check alone do not cover every workflow reader.
