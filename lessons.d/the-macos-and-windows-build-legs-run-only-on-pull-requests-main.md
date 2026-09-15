---
category: CI & GitHub Actions
---

# The macOS and Windows `build` legs run ONLY on pull requests — main builds from ubuntu shards — so "it's caught on merge" is false for anything that lives on those legs

The main-merge force build is the safety net for every *gated job*, but the matrix on `is_main` is built entirely from `ubuntu-latest` shard entries, so a PR-only leg has no backstop at all. Before trimming an OS leg, enumerate its steps gated on `runner.os` alone with no toolchain flag — `ci.yml` has three (ADJ Windows Job-object containment, ADJ macOS POSIX process-tree containment, MSVC bootstrap), and a naive trim would have dropped them on precisely the PR that edits the containment code, permanently. Grep for `if: runner.os` and check each hit for a second condition before assuming a leg is idle.
