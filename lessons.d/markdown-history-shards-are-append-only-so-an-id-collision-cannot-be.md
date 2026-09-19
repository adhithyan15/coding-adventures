---
category: CI & GitHub Actions
---

# Markdown history shards are append-only, so an id collision cannot be fixed by renaming

Two backlog shards merged minutes apart and both claimed `04980`/`HL-C409`,
because shard ordinals are chosen by reading the directory and nothing allocates
them atomically. The obvious tidy-up is to renumber one of them. It is
forbidden:

```
##[error]Markdown history shards are append-only; restore these deleted files:
code/learning/human-languages/BACKLOG.d/04980-HL-C409-FRONTMATTER-...
```

A rename is a delete plus an add, and the books workflow compares the deleted
files between the PR base and head. Any shard that exists on the base branch must
still exist on the head — `BACKLOG.d/`, `CHANGELOG.d/`, every sharded history
directory.

**What makes this worth a lesson is which checks did NOT catch it.** The
renumber passed, locally and in full:

| check | result |
|---|---|
| `npm run validate` | 21/21 |
| all twelve gates, incl. `check:doc-shards` | pass |
| the full suite (145 files) | pass |
| `check-book-compile.sh --strict malayalam` | pass |
| `scan_latex_log_warnings.py` | all six counters 0 |

The gates check a shard's **shape** — filename rank, slug, digest, heading. Only
the workflow checks a shard's **history**. So there is no local command that
reproduces this, and a change that deletes or renames a shard will look
completely clean right up until CI rejects it.

**Do this instead.** Before pushing anything that touches a sharded history
directory, confirm you have deleted nothing that the base branch has:

```sh
git diff --diff-filter=D --name-only origin/main...HEAD -- \
  code/learning/human-languages/BACKLOG.d/ \
  code/packages/typescript/human-language-data/CHANGELOG.d/
```

Empty output is the requirement. And when two shards do collide, record the
collision in a **new** shard rather than renaming an old one — appending is the
only edit the rule allows.
