---
category: Rust
---

# Scope oracle migrations by fixture boundaries instead of shared style literals

A Venn normal-position oracle preparation script tried to insert a coordinate
table beside a style literal shared by a source fixture and a direct typed
fixture. Its exact occurrence assertion refused the candidate before any
worktree source was written; that failed preparation remains separate from
compiled regression evidence.

Use the declared source-fixture geometry boundary instead, assert its exact
occurrence count, and save complete before/after bodies with a reversible byte
recipe. Keep direct typed fixtures unchanged when only generated layout moves.
