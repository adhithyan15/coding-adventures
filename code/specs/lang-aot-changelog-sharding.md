# Sharding `lang-aot`'s Changelog

## Goal

`code/packages/rust/lang-aot/CHANGELOG.md` is the repo's single worst live
merge-conflict generator. It is 6,491 lines and 522 `##` sections, newest-first,
and every PR prepends under `## Unreleased` — the same shape that made
`lessons.md` collide on every concurrent pair until it was sharded.

The file carries the evidence in plain sight: it currently holds **two separate
`## Unreleased` sections** (lines 3 and 57). Two PRs each prepended one, both
landed, and nothing reconciled them.

This applies the existing `doc-shard` machinery — the same tool that already
shards `BACKLOG.md`, `human-language-data/CHANGELOG.md`, Language Ladder,
Hindi and Script Ductus — to that file.

## Why this file, measured

`DOC_SHARD_PLANS` is explicitly a measured registry ("chosen by measurement,
not by size"). Two weaker measures were tried first and both are wrong in a
knowable way:

| measure | says | why it misleads |
|---|---|---|
| touch count over 14 days | lang-aot first, 89 touches | 89 touches spread evenly never collide; sequential edits are free |
| files touched by >1 open PR | TypeScript `package-lock.json` first | sampled one instant with 5 PRs open, right after a merge sweep |

What actually causes a conflict is two edits landing while **both branches are
alive**. So the measure used here is: per file, how many touches fall within one
typical PR-lifetime of the previous touch.

The window is derived, not chosen. Across the last 100 merged PRs the median
open→merge time is **1.19h** (p25 0.75h, p75 1.71h).

Over 21 days, ranked by touches landing within 1.19h of the previous touch, and
restricted to files still present on `origin/main`:

| close | total | ratio | path |
|---:|---:|---:|---|
| **90** | 143 | 63% | `code/packages/rust/lang-aot/CHANGELOG.md` |
| 66 | 112 | 59% | `code/specs/data/adj-facts-stdlib/CHANGELOG.md` |
| 58 | 79 | 74% | `code/specs/spice-full-implementation-plan.md` |
| 47 | 95 | 50% | `code/specs/LANG-FULL-IMPLEMENTATION.md` |
| 42 | 90 | 47% | `code/packages/rust/algol-iir-compiler/CHANGELOG.md` |

### The measure validates itself

Restricting to files still on `main` was not a detail — it is what makes the
ranking trustworthy. The excluded set is the proof:

| close | path | state |
|---:|---|---|
| 51 | `language-ladder/CHANGELOG.md` | already sharded |
| 40 | `lessons.md` | sharded, #15145 |
| 36 | `human-language-data/CHANGELOG.md` | already sharded |
| 35 | `script-ductus/CHANGELOG.md` | already sharded |
| 25 | `human-languages/BACKLOG.md` | already sharded |

Every document this repo has previously chosen to shard scores high on this
metric and has since left `main`. The measure retro-identifies the exact class
of file the repo has been fixing by hand, which is the strongest available
evidence that it is measuring the right thing — and, without the still-alive
filter, it would have ranked five finished jobs above the real one.

## Model

No new mechanism. One entry appended to `DOC_SHARD_PLANS` in
`code/packages/typescript/human-language-data/src/doc-shard-cli.ts`:

```ts
{
  path: "code/packages/rust/lang-aot/CHANGELOG.md",
  headingLevel: 2,
  newestFirst: true,
}
```

`headingLevel: 2` because the version heading IS the entry heading here
(`## 0.337.0 — …`), the same shape as the Language Ladder plan, and unlike
`human-language-data/CHANGELOG.md` where level 2 is a version banner over a
level-3 entry list.

## The gate has to reach it

**This is the part that is not merely "add a row".** `check:doc-shards` runs in
exactly one place — the `build` job of `human-languages-books.yml` — and that
job is gated:

```yaml
if: ${{ needs.detect.outputs.books == 'true' }}
```

A pull request touching only `code/packages/rust/lang-aot/` does not set
`books=true`. So a plan added for a Rust package would be governed by a check
that never runs on any PR that could break it: a registry entry enforced by
nothing, which reads as protection and is not.

Every existing plan lives under `code/learning/human-languages/` or
`code/packages/typescript/`, so this has never mattered before. It matters the
moment the registry leaves that subtree.

`check:doc-shards` therefore also runs in `ci.yml`'s `Repo-wide metadata
contracts` job, which has `needs: detect` but no `if:` and so runs on every
pull request. That job is already the home of the other repo-wide registry
checks, including `lessons.py validate`.

## Non-goals

- The other four measured candidates. The registry comment says it "grows one
  entry per follow-on PR", and one 522-section migration per change keeps each
  diff reviewable.
- Changing the `doc-shard` tool. It is already general over document paths;
  only the registry and the gate's reach need to change.
