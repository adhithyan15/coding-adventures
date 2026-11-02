---
category: Repo policy / workflow reminders
---

# On a nearly full disk, size a sparse checkout before materialising it, because code/specs alone is about 90 MB

**What went wrong.** With about 470 MB free on a disk shared with another
session, a worktree was created with the usual sparse set
(`code/packages/rust code/programs/rust code/specs code/scripts code/grammars
lessons.d`). The checkout alone wrote about 395 MB and left the disk with
1.2 MB free, which could have broken the other session's build mid-write.
`code/specs` was 91 MB of that and `code/grammars` 10 MB, and the change
needed exactly one spec file and no grammar.

**Fix.** Re-narrowed with a non-cone sparse set that names only the spec files
being edited:

```bash
git -C <wt> sparse-checkout set --no-cone '/*' '!/*/' \
  '/code/packages/rust/' '/code/programs/rust/' '/code/scripts/' '/lessons.d/' \
  '/code/specs/<the-spec-you-edit>.md'
```

That freed about 100 MB. The Rust workspace still resolves, because its
members all live under `code/packages/rust` and `code/programs/rust`.

**Do differently.** When free space is under ~1 GB, check what the sparse set
costs before running `git checkout`: `git ls-tree -r -l origin/main -- <dir>
| awk '{s+=$4} END {print s}'` gives the blob bytes for a directory (76 MB for `code/specs`; it took 91 MB on disk) without writing
anything. Find the spec files you need with `git grep -l <term> origin/main --
code/specs` and check out only those.
