---
category: Rust
---

# From a Rust package directory code scripts is three parents up not four

From `code/packages/rust/<crate>`, the repository's `code/scripts` directory is
`../../../scripts`, not `../../../../scripts`. A validation command for the
TLS alert package used the latter path and looked for a nonexistent top-level
`scripts/lessons.py` after every preceding package gate had passed.

Run repository-owned scripts from the repository root whenever practical. If
a package-working-directory command must invoke one, first count the path
components explicitly: crate to `rust`, `packages`, then `code`, followed by
`scripts/<tool>`. Keep repository validation separate from the package BUILD
chain so a path typo cannot obscure which product gates already succeeded.
