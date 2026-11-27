---
category: Testing & coverage
---

# A copied Go verifier must rename all model types when the oracle type is renamed

When adapting the scratch positional encoder into a standalone package, the
oracle's `edge` type was renamed to `testEdge`, but the copied encoder still
accepted `[]edge`. `go test` failed at compile time before any SAT case ran.

Keep the shared graph representation type explicit across the oracle and
encoder (`testEdge` here), then run `gofmt` and the package's test command after
copying between scratch and repository code. A compile failure is not SAT
evidence.
