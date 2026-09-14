# `git checkout <branch> -- <file>` destroys an uncommitted edit in that file

Immediately after the above, moving the `lessons.md` write onto its own branch,
I ran `git checkout feat/... -- lessons.md` to "bring the file across". The edit
was never committed, so that command replaced my working-tree text with the
branch's committed version and the write was simply gone — no warning, no
conflict, nothing in `git status` to notice afterwards.

Uncommitted changes already follow you across `git checkout -b`. There was
nothing to bring across; the command could only destroy. Commit first, or do
nothing.
