# `git add -A` during a merge commits conflict markers

`git commit` refuses to commit a file left in unmerged state. **`git add -A`
disarms that for every file at once**, including ones never opened.

Seen on `fix/hl-depin-corpus-counters`: a merge conflicted in two test files, a
script resolved one, and `git add -A && git commit` shipped the other with its
markers intact. A later merge then nested a fresh conflict *inside* the stale
one, so `<<<<<<< HEAD` appeared within the `origin/main` side — which reads like
main is corrupt. Main was clean; the branch was not.

It survived verification because the check was `npx vitest run <one-file>`. A
test file with conflict markers is not a failing test, it is an **uncollected
file** — a per-file run never loads it, and the suite reports the files it did
run as green.

After any conflict resolution, before committing:

```bash
git diff --name-only --diff-filter=U
```

must be empty, then sweep the branch for leftover markers, then run the **whole
package suite** rather than the file you edited. Prefer naming the resolved
files to `git add` over `-A` while a merge is in progress.
