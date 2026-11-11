---
category: Repo policy / workflow reminders
---

# Mixed repository validation commands must run Cargo steps from the Rust workspace explicitly

A combined validation command ran repository-root Python and Git checks
successfully, then appended `cargo tree` without changing into
`code/packages/rust`. Cargo correctly failed because the repository root has no
Cargo manifest.

Split mixed validation by working directory, or give every Cargo invocation an
explicit manifest path. A successful prefix does not make the final Cargo check
valid, and rerunning from the workspace is required before reporting it.
