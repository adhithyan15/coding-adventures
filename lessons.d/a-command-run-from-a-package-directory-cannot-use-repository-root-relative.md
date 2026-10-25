---
category: Rust
---

# A command run from a package directory cannot use repository-root-relative manifest paths

A strict Rust verification command changed its working directory to the new
package but kept `--manifest-path code/packages/rust/...`, which is meaningful
only from the repository root. Cargo failed before running any check. Decide
the working directory first: from a package directory use `Cargo.toml` for the
package or `../Cargo.toml` for the owning workspace; from the repository root
use the full repository-relative path. Keep every path in a compound command
consistent with that one working directory.
