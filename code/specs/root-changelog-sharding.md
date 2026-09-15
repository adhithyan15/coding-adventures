# Sharding the Monorepo Changelog

## Goal

The last document this effort migrates. `CHANGELOG.md` at the repository root
is 1,444 lines, 100 `###` entries under a `## [Unreleased]` banner — the same
shape as `human-language-data/CHANGELOG.md`, so `headingLevel: 3`.

24 close touches of 64 over 21 days.

## Two refinements to how the target was chosen

The first four migrations picked targets by contention score plus a judgement
about whether the document "is a changelog". Both halves got sharper here, and
the sharper versions are the part worth keeping.

### 1. Append-only is measurable, and it settles the prose question

"Is this an entry list or an argument?" was being answered by looking at the
filename. It is directly measurable: across recent commits, do the changed
hunks concentrate at one spot, or land throughout the file?

| document | commits | at top | median hunks | verdict |
|---|---:|---:|---:|---|
| **`CHANGELOG.md`** | 10 | **10** | 1.0 | append-only |
| `LANG-VM-NON-ALGOL-BACKLOG.md` | 10 | 10 | 1.5 | append-only |
| `package-parity-roadmap.md` | 10 | **0** | 1.0 | edited throughout |
| `spice-full-implementation-plan.md` | 10 | **0** | 2.0 | edited throughout |
| `oauth.md` | 10 | **0** | 1.0 | edited throughout |

This **confirmed two earlier calls and corrected one**. `spice-full-…` and
`oauth.md` had been excluded as "prose, probably semantic" — a guess, now
evidence at 0 of 10. And `package-parity-roadmap.md` has 64 `##` sections, so
*shape* said shardable; behaviour says no. It would have been taken on shape
alone.

### 2. A hub document costs more to shard than a leaf with the same score

`LANG-VM-NON-ALGOL-BACKLOG.md` measures append-only and scores slightly higher
than the root changelog (27 vs 24). It is **not** migrated, because the reader
count is a cost the contention score does not capture:

| document | close touches | files referencing it | references |
|---|---:|---:|---:|
| `CHANGELOG.md` (root) | 24 | **1** | 1 |
| `LANG-VM-NON-ALGOL-BACKLOG.md` | 27 | **10** | 14 |

Two of those fourteen are inside **changelog entries** — historical records of
what was true when written. Updating them would be rewriting the past to cite a
path that did not exist; leaving them makes them stale. Neither option is good,
and that is the signal to leave the document whole.

The root changelog's single reference is `README.md`'s repository-tree diagram,
updated here.

## Model

```ts
{
  path: "CHANGELOG.md",
  headingLevel: 3,
  newestFirst: true,
}
```

Level 3 rather than 2 for the same reason as `human-language-data`: level 2 is
the version banner and the hot spot is the entry list beneath `## [Unreleased]`,
where every PR prepends. The `## [0.x]` markers ride inside the entry above
them.

## A note on visibility

This removes `CHANGELOG.md` from the repository root, which is the most visible
file placement in the project. `CHANGELOG.d/_meta.md` becomes the entry point,
which the repo standard already recognises for packages — this applies the same
rule to the root. `lessons.py`-style rendering is available through
`doc-shard-cli --unshard` for anyone who wants one searchable file.

## Scope

One registry row, one shard directory, one `.gitignore` line, one entry in each
guard list, and the `README.md` tree diagram.
