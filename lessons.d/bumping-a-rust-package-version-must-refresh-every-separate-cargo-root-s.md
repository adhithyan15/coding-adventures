---
category: Rust
---

# Bumping a Rust package version must refresh every separate Cargo root's Cargo.lock that path-depends on it

**What went wrong.** LGE-RUST-HARDEN (#20041) bumped `rust/logic-gates` from 0.1.0
to 0.2.0 and updated the workspace lockfile, but `code/programs/rust/vos-capacity-probe`
is a *separate* Cargo root with its own `Cargo.lock` that path-depends on logic-gates.
CI's "Repo-wide metadata contracts" job resolves all ~159 Cargo roots with `--locked`
and failed: "Cargo.lock is stale or does not resolve ... Updating logic-gates v0.1.0 -> v0.2.0".
Local `cargo test` in the package and its workspace passed, because nobody builds that root.

**Fix.** `(cd code/programs/rust/vos-capacity-probe && cargo update -p logic-gates --offline)`
and commit the one-line lockfile change.

**Do differently.** After any `version =` bump in a Rust `Cargo.toml`, find every lockfile
that names the crate and check each resolves:

```sh
grep -rl 'name = "<crate>"' --include=Cargo.lock code | while read f; do
  (cd "$(dirname "$f")" && cargo metadata --locked --offline --format-version 1 >/dev/null) \
    || echo "STALE: $f"
done
```

Fix each stale root with `cargo update -p <crate> --offline` before pushing.
