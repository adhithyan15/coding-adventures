---
category: Repo policy / workflow reminders
---

# Run the repository build tool from current source

The checked-in `code/programs/go/build-tool/build-tool` executable can lag the
Go source in the same directory. Invoking that binary during validation
reported stale dependency-metadata failures that the current implementation
does not necessarily reproduce. From the repository root, set an explicit
repository path and run the validator inside its Go module:

```bash
task_repo_root=$(pwd)
go -C code/programs/go/build-tool run . -root "$task_repo_root" -dry-run
```

Use fail-fast shell semantics and treat only that source-built result as the
current gate.
