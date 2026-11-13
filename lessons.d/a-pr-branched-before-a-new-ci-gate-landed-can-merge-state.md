---
category: Repo policy / workflow reminders
---

# A PR branched before a new CI gate landed can merge state that fails that gate

#19220 made `code/scripts/cargo_lockfiles.py check` a CI gate and started
tracking `code/packages/rust/Cargo.lock`. Three PRs that were already open
merged after it, each changing the Rust workspace's dependency graph without
touching the lock:

- #19316 bumped `x509-ocsp-reviewed-server-revocation-binding` to 0.5.0;
- #19314 gave `gf256` a `capability-runtime-conformance` dev-dependency;
- #19310 gave `lzw` new dev-dependencies.

Their CI ran against a base without the gate, so all three went green. On
main, every later PR that merged main failed `Repo-wide metadata contracts`
with "Cargo.lock is stale or does not resolve". The fix was
`(cd code/packages/rust && cargo update --workspace)`, which changes only
workspace-member lines, in its own PR (#19322).

**Do differently.** When a new repo-wide gate merges, an open PR's green CI
says nothing about that gate. Before a PR that touches a `Cargo.toml` merges,
merge main into it, or run the gate locally:
`python3 code/scripts/cargo_lockfiles.py check --changed-since origin/main`.
In a sparse checkout, a "Cargo.lock is missing" line for a root outside the
sparse set is an artifact; read only the lines for the roots you changed.

**A second trap behind the first.** Once the Rust lock was fresh, the same
gate failed on the seven `gf256` wrapper roots (Elixir, Lua, Perl, Python,
Ruby, TypeScript, WASM): "Cargo.lock is missing but the project resolves
external crates". They resolve none. `cargo_lockfiles.py` followed
`dev-dependencies` transitively, and #19314 had given `rust/gf256` a dev-only
path dependency whose own dependencies include `libc`. Cargo resolves
dev-dependencies only for the root package and workspace members, never for
a path dependency reached transitively; `cargo generate-lockfile` in
`code/packages/wasm/gf256` produced a lock with no registry entries at all.
The script now follows dev-dependencies only where Cargo does. A gate that
models a tool's semantics should be checked against the tool itself
(`cargo generate-lockfile`) before trusting its verdict.
