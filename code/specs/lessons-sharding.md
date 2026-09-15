# Lessons Sharding

## Goal

`lessons.md` is the repo's accumulated record of engineering failures and their
durable fixes. CLAUDE.md requires every PR that hits a CI failure to add an
entry, and requires every implementation to read the file first. Both rules are
working as intended. The file is not.

At 7,309 lines and 640KB it had become a **structural merge-conflict generator**.
Every branch appends, almost always near the end or at the end of the same
topical bucket, so any two concurrent PRs collide. Measured on the four PRs open
when this was written, `lessons.md` was the *only* conflicted file in two of
them and one of two in a third. The conflicts carry no information — both sides
are additive and the resolution is always "keep both" — but each one blocks
auto-merge and costs a round trip.

The size hurts the read rule too. "Read `lessons.md` cover to cover" is not an
instruction anyone can follow against 640KB, so in practice it is skimmed, which
is the failure mode the file exists to prevent.

Sharding fixes both: **a new lesson becomes a new file**, so concurrent lessons
cannot conflict at all, and lessons become individually addressable by path.

## Model

`lessons.d/` holds one Markdown file per lesson.

```
lessons.d/
  _meta.md                     entry point: the scheme and how to add a lesson
  <slug>.md                    one lesson
```

The directory is **flat**, matching the existing `CHANGELOG.d/` precedent in
this repo (`code/packages/typescript/human-language-data/CHANGELOG.d` holds 637
flat files). Grouping lives in frontmatter, not in directory structure, so that
filing a lesson never requires inventing a directory and never has to be
revisited when categories drift.

Each shard is:

```markdown
---
category: Rust
---

# A tail expression holding a MutexGuard temporary compiles locally but fails CI

<body>
```

`category` is optional. When present it must name one of the categories listed
in `_meta.md`; when absent the lesson renders under `Uncategorised`.

### Filenames are the identity

The slug is derived from the title and is the lesson's only identifier. There is
deliberately **no `id:` field and no ordinal prefix**, because this repo has
been bitten by both:

- **Ordinal prefixes are positional.** A `(index + 1) * 10` scheme elsewhere in
  this repo renamed 21 files on a single insertion, and every rename is a
  conflict for every branch holding that file.
- **A separate `id` field collides silently.** Two branches once wrote
  `MR-EXT-038` into differently-named files; git saw two unrelated new files,
  merged both, and the duplicate was invisible from either branch alone.

Making the filename the identity closes both. Two branches writing genuinely
different lessons produce different filenames and merge cleanly. Two branches
writing the *same* lesson produce the same filename, and git raises a real
conflict — which is the correct outcome, because they need reconciling.

### `lessons.md` is generated, not committed

The aggregate view is still useful for reading in bulk, so `lessons.py render`
rebuilds it on demand. It is **gitignored**, for the same reason
`human-language-data` ignores its rendered `CHANGELOG.md`: a committed aggregate
would reintroduce exactly the shared file this change removes.

## CLI

`code/scripts/lessons.py`:

| Command | Behaviour |
|---|---|
| `validate` | Every shard parses, has a title, has a known category, and no two shards share a slug. Non-zero exit on any failure. |
| `render [path]` | Writes the aggregate, grouped by category in `_meta.md` order, then alphabetically by title. Defaults to `lessons.md`. |
| `index` | Prints `category<TAB>slug<TAB>title` for every shard. |
| `new "<title>" [--category C]` | Creates `lessons.d/<slug>.md` with frontmatter and a heading, and prints the path. Refuses an existing slug. |

## Migration

The one-time split is verified by reconciliation against the whole input, not by
sampling: every non-whitespace character of the source body must appear in
exactly one shard. The only permitted loss is the 23 bucket `## Heading` lines,
whose content moves into each shard's `category` field.

Measured on the source file at migration time:

| | |
|---|---|
| `##` sections | 253 |
| of those, topical buckets | 23 |
| lessons extracted from buckets | 264 |
| whole-section lessons | 230 |
| **shards written** | **494** |
| dense chars in, dense chars out | 536,801 → 536,369 |
| delta | 432, exactly the 23 bucket headings |

Buckets are an **explicit allowlist** in the migration script, not a heuristic.
A first pass inferred "is this title a label or a claim" from title length and
word count and misfiled two obvious buckets — `Compiler / VM / language
pipeline` (31 bullets) and `QR / format-marker / file-format specifics`. A third,
`Gradle / JVM`, reads like a bucket but holds zero top-level bullets and
structures itself with `###` subsections; treating it as one silently dropped ten
prose lines. There are only ~23 buckets in the file, so enumeration is cheap and
reviewable in a way a threshold is not.

## Consequences for the repo rules

CLAUDE.md rule 7 ("read `lessons.md` cover to cover") becomes: read `_meta.md`,
then the categories relevant to the work at hand. Rule 8 ("add it to lessons.md
immediately") becomes `lessons.py new`.

Neither rule weakens. The read rule gets *more* followable, because a category is
a tractable amount of text where 640KB was not.
