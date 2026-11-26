---
category: CI & GitHub Actions
---

# A Rust workspace consumer in a program build front needs a repository boundary entry

The `vos-model-sat` program has a Cargo manifest of its own, but its `BUILD`
also references the repository Rust workspace manifest. The
`rust-root-workspace-manifest-cross-consumers` rule is intentionally textual:
any matching BUILD root under `code/packages` or `code/programs` is part of the
set, regardless of whether its command later resolves through another manifest.

When a BUILD front adds or removes a matching Cargo workspace reference, run
`python3 code/scripts/build_tool_conformance.py sync-boundary` from the same
change. This regenerates the canonical boundary fixture, C# and Swift
projections, digest pins, count pins, and affected corpus cases from the
repository. Then run `sync-boundary --check` and the
`test_repository_source_input_boundary_is_closed_and_canonical` test before
committing. Do not hand-edit just the fixture or copy a digest into a subset of
readers.
