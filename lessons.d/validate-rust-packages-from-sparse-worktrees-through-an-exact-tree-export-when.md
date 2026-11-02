---
category: Rust
---

# Validate Rust packages from sparse worktrees through an exact-tree export when omitted workspace members have no targets

A package-local `cargo test -p NAME` still discovers and parses the parent
workspace. In a sparse worktree, patterns that retain every member
`Cargo.toml` but omit unrelated `src/` directories can therefore fail with
"no targets specified" before Cargo reaches the selected package. That is a
checkout-shape failure, not evidence against the package under review.

Before running a Rust BUILD front in a sparse worktree, inspect the sparse
patterns and the parent workspace. If member sources are intentionally absent,
export the exact reviewed commit's tracked Rust workspace and required fixture
paths into a fresh temporary directory, then point Cargo at the exported
workspace manifest. Verify the exported revision first, retain the normal lock
and dependency policy, and remove or trash the temporary tree immediately
after validation. Do not "fix" the review branch by fabricating target files,
editing unrelated manifests, or treating a reduced ad hoc workspace as the
repository's real dependency graph.
