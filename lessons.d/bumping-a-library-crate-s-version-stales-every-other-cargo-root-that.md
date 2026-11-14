---
category: BUILD files & dependency management
---

# Bumping a library crate's version stales every other Cargo root that path-depends on it

The x25519 Warrant pilot (#19352) made `clamp_scalar` public and bumped
`coding_adventures_x25519` from 0.1.0 to 0.2.0. The main workspace lock
(`code/packages/rust/Cargo.lock`) was refreshed, but two separate Cargo
roots, `code/programs/rust/vault-pm-cli` and `vault-pm-cli-drill`, also
path-depend on x25519 and carry their own `Cargo.lock`. Their locks still
recorded 0.1.0, so `Repo-wide metadata contracts` failed
`cargo_lockfiles.py check` ("Cargo.lock is stale or does not resolve").

Fix: `cargo update --workspace` in each affected root; the diff is the one
version line.

Do differently: after bumping a crate's `version`, find every lock that
names it, `git grep -l '<crate_name>' -- '*Cargo.lock'`, and run
`cargo update --workspace --locked` in each of those roots before pushing.
`cargo_lockfiles.py check` needs the full tree, so in a sparse worktree the
per-root `--locked` run is the quick local substitute.
