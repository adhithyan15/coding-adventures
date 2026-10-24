---
category: Repo policy / workflow reminders
---

# Run the repository build tool from current source

The checked-in `code/programs/go/build-tool/build-tool` executable can lag the
Go source in the same directory. Invoking that binary during validation
reported stale dependency-metadata failures that the current implementation
does not necessarily reproduce. Run the validator with
`go run ./code/programs/go/build-tool` from the repository root, use fail-fast
shell semantics, and treat only that source-built result as the current gate.
