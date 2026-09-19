## HL-C411 — two backlog entries share the id HL-C409, and the append-only rule means neither can be renamed

**Status: OPEN.** Two entries in this directory carry the ordinal `04980` and
the id `HL-C409`:

**Numbering note:** active PR #15593 already reserves `HL-C410`, so this entry
takes the next free id, `HL-C411`, instead of recreating the same race.

| file | subject | merged by |
|---|---|---|
| `04980-HL-C409-FRONTMATTER-UNQUOTE-...` | `unquote()` does not decode escapes | #15582 |
| `04980-HL-C409-MALAYALAM-A1-READING-...` | A1 reading stops seventeen words short | #15578 |

They merged within minutes of each other. Shard ordinals and ids are chosen by
**reading the directory**, so two branches opened at the same time both saw
`04970/HL-C408` as the highest and both took the next one. Nothing in the
tooling allocates them atomically, and nothing rejects a duplicate.

**Two things are broken by it.** `HL-C409` no longer names one entry, so a
citation is ambiguous; and `_meta.md` promises entries are *"ordered
newest-first by descending ordinal"*, which cannot hold when two share an
ordinal — their relative order is whatever the filesystem says.

**THE OBVIOUS FIX IS FORBIDDEN, AND THAT IS THE USEFUL PART OF THIS ENTRY.**
Renaming one of them is a delete plus an add, and the books workflow enforces
that Markdown history shards are **append-only**:

```
##[error]Markdown history shards are append-only; restore these deleted files:
code/learning/human-languages/BACKLOG.d/04980-HL-C409-FRONTMATTER-...
```

This was learned by doing it: a renumber to `04990/HL-C410` passed `npm run
validate`, all twelve gates including `check:doc-shards`, the full suite, the
strict book compile and the LaTeX warning scanner, and was rejected by CI. The
gates check shard *shape*; only the workflow checks shard *history*. So the
collision cannot be tidied away after the fact by anyone, and this entry exists
instead of a rename.

**How to cite these two until it is closed.** By filename, not by id. The
frontmatter entry is `04980-HL-C409-FRONTMATTER-...`; the reading entry is
`04980-HL-C409-MALAYALAM-A1-READING-...`.

**What closing this needs.** Not a rename. Either:

- teach the shard tooling to allocate an ordinal and id that are unique against
  the directory **and** against open PRs — a per-branch suffix, or an id derived
  from the PR number rather than a running count; or
- add a check that *fails* on a duplicate id or ordinal at author time, so the
  second branch is told to pick again while renaming is still free.

The second is much cheaper and would have caught this one, because the collision
only became unfixable at merge. Until then, expect this to recur: it is a
function of two curriculum branches being open at once, which is the normal
state of this repo.
