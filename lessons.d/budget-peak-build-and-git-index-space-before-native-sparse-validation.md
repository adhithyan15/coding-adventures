---
category: CI & GitHub Actions
---

# Budget peak build and Git index space before native sparse validation

Creating a VisiCalc successor checkout with broad inherited sparse paths filled
the disk while extracting specs/data. A later compiler build left too little
space for Git to rewrite its index. A narrower checkout recovered space, but
excluding every non-Markdown spec also omitted mosaic-compile.json, which the
CLI needs at runtime even when the executable has compiled.

Use worktree add --no-checkout, initialize a narrow pattern set, then populate
with read-tree -mu HEAD. Include runtime CLI specs and shared Mosaic packages.
Reserve space for Rust targets, native obj/bin and a temporary Git index, not
just source files. After successful generation/tests, cargo clean in the isolated
checkout can free regenerable targets before native compilation. Preserve native
binaries, evidence logs and unrelated work; verify repository status after a
failed index write and reapply the intended sparse patterns.
The first version also used an unregistered lesson category and failed CI. Choose
from lessons.d/_meta.md and run lessons.py validate before publishing.
