---
category: Repo policy / workflow reminders
---

# In a no-checkout worktree, configure sparse-checkout before anything that populates the tree; a reset --hard first checks out the whole repo

**What went wrong.** With about 1.2 GB free on a disk shared by several agents
(2026-10-27, #18463), a plain `git worktree add` ran out of space part-way
through: a full checkout of this repo is larger than that, mostly
`code/learning/human-languages` (about 690 MB of blobs). Git cleaned the failed
worktree up. The retry used `git worktree add --no-checkout`, but then ran
`git reset --hard origin/main` *before* `git sparse-checkout set`. With no
sparse patterns configured yet, the reset populated the entire tree and took
the shared disk to 0 bytes free, so every other agent's writes on it failed
until the worktree was removed again.

**The fix.** Order matters. Populate the tree only after the patterns exist:

```bash
git worktree add --no-checkout .claude/worktrees/<name> <branch>
cd .claude/worktrees/<name>
git sparse-checkout init --cone
git sparse-checkout set code/packages/rust code/specs code/scripts lessons.d
git read-tree -mu HEAD        # populates only the cone
```

That gave a 385 MB working tree for Rust work. Building `lang-aot` also needs
`code/grammars` (the lexer crates' `build.rs` read `../../../grammars/*.tokens`),
so add it with `git sparse-checkout add code/grammars` before building.

**Next time.** On a tight or shared disk, measure first
(`git ls-tree -r -l origin/main | awk '{s+=$4} END {print s}'`), and treat any
command that writes the working tree (`reset --hard`, `checkout`, `switch`,
`read-tree -u`) as a full checkout unless sparse patterns are already set.
Related: `a-fresh-sparse-worktree-may-hide-lessons-d-until-the-path-is`,
`expand-sparse-checkout-for-build-tool-go-dependencies-before-compiling`.
