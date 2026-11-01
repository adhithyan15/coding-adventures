---
category: Rust
---

# Formatting paths must be relative to the command working directory

`rustfmt` was launched from `code/packages/rust` with paths that still began at
the repository root, so it looked for a duplicated `code/packages/rust` prefix
and failed before formatting anything. Choose the command working directory
first, then express every explicit file path relative to that directory (or use
absolute paths). Keep the focused formatter and the Cargo command in one
`set -e` block only after both path conventions agree.

The same working-directory rule applies to workspace Cargo commands: this
repository's Rust workspace manifest lives in `code/packages/rust`, not the
repository root. Run Cargo there (or pass the manifest path explicitly). Scope
formatting to the changed package with `cargo fmt -p <package> --check` because
`cargo fmt --all --check` also reports unrelated pre-existing formatting debt
across this monorepo. A workspace-wide `cargo check --workspace` is still the
appropriate compatibility check. Use `set -e` when a validation batch must stop
on the first failed command rather than continuing with a partial result.
