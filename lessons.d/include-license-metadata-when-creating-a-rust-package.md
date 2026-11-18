---
category: Repo policy / workflow reminders
---

# Include license metadata when creating a Rust package

A new Rust package can compile and pass focused tests while still omitting the
repository's required package-license declaration. When adding a `Cargo.toml`,
copy the current metadata convention from directly related repository packages
and verify `cargo metadata` reports a non-null license or license file before
requesting review. For the repository's MIT-licensed TLS packages, declare
`license = "MIT"` explicitly.
