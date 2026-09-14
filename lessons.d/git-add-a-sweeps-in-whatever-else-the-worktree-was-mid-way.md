# `git add -A` sweeps in whatever else the worktree was mid-way through

Two work streams in one worktree — a deck-list fix and a new glyph-parser crate
— and `git add -A` put ten files of the half-finished crate into the deck-list
commit. The PR then carried an unrelated 6,000-line crate, and splitting it
afterwards meant rebuilding both branches from `origin/main` file by file.

`git status` before committing shows this immediately; the fix is to name the
paths. Cheap to avoid, tedious to undo.
