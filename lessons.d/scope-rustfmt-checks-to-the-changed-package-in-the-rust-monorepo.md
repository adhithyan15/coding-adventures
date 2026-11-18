---
category: Repo policy / workflow reminders
---

# Scope rustfmt checks to the changed package in the Rust monorepo

Running `cargo fmt --all --check` from the Rust workspace audits every package,
including unrelated historical formatting. The resulting report can be
enormous and obscures the changed package without providing a proportional
gate. Use `cargo fmt -p <package> --check` for a bounded package slice, then
pair it with the package BUILD, Clippy, rustdoc, and diff checks required by
the repository workflow. Do not rewrite unrelated packages to satisfy a local
slice's formatting gate.
