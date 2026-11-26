---
category: BUILD files & dependency management
---

# Cargo loads the dev-dependency manifests of every path dependency, so a sparse checkout needs their whole chain

Building `warrant-corpus-gen` in a sparse worktree failed after `warrant-grammar` gained a path **dev**-dependency on
`ct-compare`: cargo resolves a path dependency's **own** `[dev-dependencies]` manifests too (`ct-compare` dev-depends on
`capability-runtime-conformance`, which needs `capability-cage`, `capability-os-sandbox`, `json-parser`, ...), so
filtering the workspace `members` to the present crates is not enough; every crate reachable through any dependency edge,
dev edges of path dependencies included, must be in the checkout, though none of their sources is compiled.

What to do: when a build says `failed to load manifest for dependency X`, add X's directory to the sparse checkout and
repeat until it resolves (a dozen small crates here), instead of editing a `Cargo.toml` to drop the edge. The first
failure names only the first missing crate, so expect several rounds.
