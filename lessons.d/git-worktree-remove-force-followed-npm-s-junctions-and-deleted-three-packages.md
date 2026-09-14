# `git worktree remove --force` followed npm's junctions and deleted three packages in ANOTHER worktree

To test whether a failing test was pre-existing, a throwaway worktree was created at `origin/main`
and seeded by copying `node_modules` from the working worktree, to skip a slow `npm ci`:

```
cp -r $WORK/packages/typescript/$p/node_modules $TEMP/packages/typescript/$p/node_modules
```

The identity test worked and answered the question. Then:

```
git worktree remove --force $TEMP
```

…and **three package source directories vanished from the WORKING worktree** —
`paint-instructions`, `paint-vm`, `pixel-container`, 29 tracked files, none of them anywhere near
the temporary tree.

**Why.** These packages depend on each other by `file:` reference, so npm materialises
`node_modules/@coding-adventures/<pkg>` as a **directory junction** pointing at the sibling
package's real directory. Copying `node_modules` copied junctions that still pointed **into the
original worktree**. `git worktree remove --force` deletes the tree recursively, walked into a
junction, and deleted the target's contents.

This is the same hazard `shard-cli.ts` documents for `existsSync` + `rmSync` — *"entries reached
THROUGH a symlinked parent report `isSymbolicLink() === false`"* — arriving from a completely
different direction. There, the guard was `lstatSync`. Here, there is no guard to add: the mistake
was **copying a tree containing junctions to somewhere it would later be recursively deleted.**

**Rules:**

- **Never `cp -r` a `node_modules` that contains `file:` deps between worktrees.** The junctions do
  not get rewritten and they point back at the source. Run `npm ci` in the new tree, or seed it
  with `cp -r --dereference` / `robocopy /SJ`, which copies the junction as a junction rather than
  following it.
- **Before `git worktree remove --force`, check for junctions:** on Windows,
  `cmd //c dir /AL /S <path>` lists reparse points. If any point outside the tree, delete
  `node_modules` first and then remove the worktree.
- **It was recoverable only because the files were tracked and unmodified.** `git checkout -- <dirs>`
  restored all 29. Had those directories held uncommitted work, `--force` would have destroyed it
  with no warning and no prompt.

The wider point: the identity test was the right call and produced the right answer. **The
shortcut taken to make it cheap was the dangerous part**, and it was dangerous in a way that had
nothing to do with what was being tested.
