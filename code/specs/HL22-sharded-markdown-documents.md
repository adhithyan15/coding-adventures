# HL22 — Sharded Markdown documents: the last two serialization points

**Status:** specification, 2026-08-25

**Extends:** HL21 (*Sharded curriculum ledgers*), whose convention this applies
to prose. HL21 §2 defines `X.json` → `X.d/`; this defines `X.md` → `X.d/`, and
does not change anything HL21 already migrated.

---

## 1. Why this spec exists

HL21 removed the conflict points in the curriculum's **data**. It did not touch
the two files that every human-languages author edits regardless of what they
are working on:

| File | Sections | Touched by, of the last 200 human-languages commits |
| --- | ---: | ---: |
| `code/learning/human-languages/BACKLOG.md` | 107 | **100** |
| `code/packages/typescript/human-language-data/CHANGELOG.md` | 436 | **75** |
| `<track>/CHANGELOG.md` (23 files) | — | 4–11 *each* |

The last row is the control, and it is the reason this spec names exactly two
files. The per-language changelogs are **already partitioned by track** and have
never been a conflict point. Sharding a file that does not conflict buys nothing
and costs a directory, so HL22 leaves all 23 of them alone.

### 1.1 The corroborating experiment

PR #12690 — HL21 §5.1, the `chapters.json` sharding — went `DIRTY` three
separate times while two concurrent Spanish tranches were in flight. Every
single time, the conflicting file was **only** `CHANGELOG.md`. Not one of its
~4,000 shard files ever conflicted.

That is the whole argument in one data point: the sharding pattern works, and
these two files are the part that had not had it applied yet. A PR whose entire
purpose was to remove a conflict point was itself blocked, three times, by a
conflict point it did not remove.

### 1.2 What this unblocks

The goal is concrete: run 5–7 parallel level-authoring agents (A2, pre-B1, B1,
B2, pre-C1, C1, C2) without them colliding. Every one of them appends a backlog
finding and a changelog entry. Today all 5–7 collide on both files; the merge is
mechanical, requires no judgement, and still requires a human — the worst
possible combination, and it scales quadratically in the number of authors.

---

## 2. The convention, and where it differs from HL21

For a shardable document at `X.md`, the sharded form is a sibling directory
`X.d/` holding one `*.md` file per section, plus `_meta.md` for the preamble:

```text
code/learning/human-languages/BACKLOG.md      the monolith (generated)
code/learning/human-languages/BACKLOG.d/      the shards   <- source of truth
  _meta.md                                    title + provenance note
  01070-HL-C10E-URDU-CLOSES-THE-NEXT-SHARED-GLYPH-GAP-9b4a2368.md
  01060-HL-C10D-TAMIL-CLOSES-THE-SHORT-E-COMPOSITION-GAP-b8bd6ec5.md
  …
```

Everything in HL21 §2 carries over unchanged: `.d` is the Unix `conf.d` idiom;
order is sorted filename order compared by **code unit**, never `localeCompare`;
`X.d/` present but empty is an **error**, not an empty document; `_meta.md` is
**required**, never defaulted; symlinked shard directories, shards and monoliths
are **refused** rather than followed.

Three things are genuinely different, and each is different for a reason.

### 2.1 There is no serialization, so the round trip is free

HL21's byte-exactness required work: `JSON.stringify(value, null, 2) + "\n"` had
to be *verified* to reproduce the committed bytes, and §2.5 had to require the
sharded array be the last top-level key so it would land back where it started.

None of that applies to prose, because HL22 **never re-serializes anything**. It
partitions the file's bytes at heading boundaries and writes the slices out
verbatim; rebuilding is `concat`. `concat(split(x)) === x` for the same reason
that cutting a rope and taping it back together gives you the rope, and
`splitDocument` asserts it on every run rather than trusting it.

**Consequence: this migration introduces no normalization at all.** The
regenerated monoliths are byte-identical to the pre-migration files — verified
by `git diff --exit-code`, not by argument.

### 2.2 The ordinal is a RECENCY RANK, because these documents are newest-first

This is the substantive design problem, and HL21's worked examples do not have
it. A spine node or a chapter is **appended**: it takes the next ordinal and
nobody renames anything.

A backlog and a changelog are **prepended**. Under plain ascending filename
order the newest entry would need the *smallest* ordinal, so every author would
be reaching downward into a shrinking gap — two agents would both compute "one
less than the current minimum", and the numbering would run out at zero.

The rule that actually matters in HL21 §2.1 is that the order be
**deterministic and locale-free**, not that it be ascending. Descending code-unit
order is exactly as reproducible. So for a newest-first document the ordinal is
a recency rank:

```text
highest ordinal  =  topmost section  =  newest
lowest  ordinal  =  bottom  section  =  oldest
```

and the join walks it downward. Adding an entry becomes the ordinary append case
again: `max + stride`, no neighbour renamed, no gap to exhaust.

`_meta.md` is found **by name** and emitted first in both directions, rather than
relying on where `_` happens to sort. Under ascending order `_` (0x5F, above
every digit) would trail; under descending order it would lead by luck rather
than by rule, and luck is not a specification.

The plan records this as `newestFirst`, per document, so an oldest-first document
migrated later gets ordinary ascending order without a special case.

### 2.3 The shard filename has three parts, and the identity is the digest

```text
04360-ADDED-SOURCE-VERIFIED-URDU-6b06ea3c.md
^^^^^ ^----------------------^ ^------^
rank   human-readable slug      heading digest
```

- **Rank** orders the join. Two agents appending at the same moment will both
  compute the same rank, and that is fine: a tie is broken by the rest of the
  name, and two entries authored the same day have no true relative order to
  lose.

- **Slug** is for the human running `ls`. It is ASCII-folded, and it is
  therefore **not unique** — `### Added - source-verified Tamil ர` and
  `### Added - source-verified Tamil த` fold to the *same* slug, because the
  only thing distinguishing them is a character no filename should carry across
  NTFS, APFS and ext4. A tool that deduplicated on the slug would silently drop
  half this changelog.

- **Digest** is the identity: the first 8 hex of SHA-256 over the raw heading
  line. Two different headings give two different filenames, full stop.

This is HL21 §2.6 restated for a domain where ids do not exist. HL21 could
derive a filename from an authored `id` field and validate it against
`^[A-Z][A-Z0-9-]*$`. A prose section has no id — `BACKLOG.md` proves it, since
106 of its 107 sections start `## HL-…` and one does not (`## Three rules this
work keeps re-deriving`). An id-based scheme would have had to special-case that
section or reject the file. Deriving from the heading text and validating the
*result* removes the question.

The digest is over the **heading**, not the section body. Hashing the body would
make every prose edit rename its own file, so the next `--shard` would emit a
mass rename — and a mass rename is a mass merge conflict, which is the thing
this work exists to remove. Headings change rarely; bodies change constantly.

### 2.4 Fenced code blocks

`^## ` inside a fenced code block is not a heading. Splitting on it would cut a
code block in half — **and the two halves would still concatenate back to the
original file**, so `--check` would pass while the shards were nonsense.

That is the dangerous class of bug in a partition-based sharder, and it is
exactly the class its strongest guarantee cannot catch: byte-exactness tells you
nothing about whether the cut was in a sensible place. So the splitter tracks
fence state, and this is the one behaviour that needs its own test rather than
riding on the round trip.

`BACKLOG.md` has 5 fenced blocks and none contains a `## ` line; `CHANGELOG.md`
has none at all. Both were measured. The guard is for the block somebody adds
next week.

---

## 3. Why the monoliths survive, and why the reason is NOT HL21's

HL21 §4.1 kept `core/spine.json` because `language-ladder` statically imports it
into a browser bundle with a 500 kB eager budget. The obvious guess is that the
same applies here. **It does not, and it was checked rather than assumed:** every
Vite glob in `language-ladder/src` requires a subdirectory segment
(`*/lessons/*.md`, `*/curriculum.json`, `core/generated-book-hashes/*.json`, …),
so neither of these files can match one. Neither is bundled.

In fact **nothing in the repository reads either file programmatically.** No
loader, no test, no fixture, no shell script, no Python/Ruby/Go tooling, no
workflow. Both are pure prose. The curriculum-root scans in `loader.ts` all gate
on a specific file inside each directory (`curriculum.json`, `chapters.json`,
`book/book.tex`, `lessons/`, `book/preamble.tex`), so the new `BACKLOG.d/`
directory sitting at the curriculum root is not mistaken for a language track —
also checked, because that would have been a silent corpus-wide failure.

They are kept for two prosaic reasons:

1. **Seven documents link `BACKLOG.md` by relative path**, two of them to a
   section anchor (`../BACKLOG.md#findings-from-hl-c30`). Nothing in CI validates
   Markdown links, so deleting the file would break them *silently* — the worst
   way for a link to break. Because sharding preserves heading text byte-exactly,
   the anchors keep working.
2. **`CLAUDE.md` requires every package to carry a `CHANGELOG.md`.** That rule is
   policy rather than machinery — there is no changelog gate anywhere in
   `.github/workflows/`, which was checked — but it is the convention, and a
   reader opening the package expects the file.

Per HL21 §3, a derived file that nothing verifies is worse than no file. So both
monoliths are gated:

```sh
npm run shard:docs    <path>   # split a document into X.d/
npm run unshard:docs  <path>   # rebuild the monolith from X.d/
npm run check:doc-shards       # assert the two agree; what CI runs
```

`check:doc-shards` is wired into `.github/workflows/human-languages-books.yml`
and `code/scripts/verify-human-languages.sh`, beside `check:shards`.

### 3.1 What `--check` deliberately does not check

HL21's JSON `--check` compares shard **filenames** against what `--shard` would
emit today. HL22's does not, and the difference is deliberate: HL21 §2.2 promises
that an entry wedged in by hand as `0015` passes without a renumber, and a
filename comparison breaks that promise. (It is worth noting that HL21's
implementation has this inconsistency live — `shard-cli.ts`'s reverse check
requires canonical names while its docstring promises the opposite. Filed, not
fixed here.)

What HL22 checks instead is the failure the byte comparison genuinely cannot
see: a file in `X.d/` that the join **never looked at**. A missing or corrupted
`*.md` shard needs no separate check, because the rebuilt document *is* the
shards. But a shard saved as `.markdown`, `.txt`, or `.md.orig` — the last being
what a botched merge leaves behind — sits in the directory looking like content
and contributes nothing, while the document still rebuilds cleanly. That is
exactly why nobody would notice.

---

## 4. Working in a sharded document

**To add an entry:** create ONE new file in `X.d/`. Give it an ordinal above the
current maximum (`ls X.d/ | tail -2`), a readable slug, and any 8 hex digits —
`--shard` will canonicalise the name if it is ever re-run, and `--check` does not
care. Then run `npm run unshard:docs <path>` and commit both.

**Never hand-edit the monolith.** Edit the shard and regenerate.

**Never hand-merge the monolith.** Hand-merging a generated file produces bytes
no generator would emit, and the next `--check` fails for reasons nobody can
read. On a conflict, take either side and regenerate:

```sh
git checkout --ours code/learning/human-languages/BACKLOG.md
npm run unshard:docs code/learning/human-languages/BACKLOG.md
```

`--ours` versus `--theirs` does not matter, and that is the point: the monolith
carries no information the shards do not, so either starting point regenerates to
the same bytes. This supersedes the older "union both sides of `CHANGELOG.md`"
convention *for these two files only* — union-merging is still right for the 23
per-language changelogs, which are not generated.

### 4.1 What this actually buys, measured

Two branches were built off one base, each adding one backlog entry and one
changelog entry — the exact shape of two parallel level-authoring agents — and
merged. Both agents independently chose the **same ordinal**, which is the
expected case and the one the design has to survive.

| File | Result |
| --- | --- |
| `BACKLOG.d/01090-AGENT-BB-5104222c.md` | merged clean, no conflict |
| `CHANGELOG.d/04380-AGENT-BB-ee659fe1.md` | merged clean, no conflict |
| `BACKLOG.md` (generated) | conflict |
| `CHANGELOG.md` (generated) | conflict |

The digest kept the same-ordinal filenames apart, so git never saw a question.
After the two-command resolution above, **both** agents' entries were present in
both regenerated files and no conflict markers remained.

The honest summary: this does not make the merge conflict-free, it makes it
*content-free*. Before, resolving meant merging prose by hand with a live risk of
dropping somebody's entry — which is how this `CHANGELOG.md` acquired three
"Unreleased" sections (§5.1). Now the conflict is confined to a derived file that
carries no information, and the resolution cannot lose anything because the
content was never in the conflicting file.

**`--shard` is a migration tool, not a routine step.** Running it renumbers to
the canonical stride, which is a mass rename and therefore a mass merge conflict.
Run it when a document is quiet, or not at all.

---

## 5. Findings recorded, not fixed

Both are pre-existing and neither is caused by this migration. HL21 §6's rule
applies: an ordering quirk the tooling did not create is *a finding to report*,
not a file to quietly fix into agreement with the generator.

### 5.1 `CHANGELOG.md` has three separate "Unreleased" sections

At the pre-migration line numbers: `## Unreleased` (line 3), a second
`## Unreleased` (line 952), and `## [Unreleased]` (line 1021). Two of the three
are almost certainly the residue of exactly the bad merges this spec exists to
prevent — someone resolved a conflict by keeping both sides, heading included.

They survive the migration untouched, riding inside the frozen entry above each,
because byte-exactness is the higher priority. Consolidating them is a separate,
deliberate commit that should be made when the file is quiet, and it should
decide what the second and third sections were meant to be released *as*.

### 5.2 Neither file's `.d/` directory reduces CI cost

Both files sit inside directories that CI treats as opaque path prefixes:
`human-languages-books.yml` triggers on `code/learning/human-languages/**` and
on `code/packages/typescript/human-language-data`, and the build tool treats a
shell-style `BUILD` package as "any file under this path rebuilds it". So a
changelog-only edit still triggers a 23-book XeLaTeX compile and a full
`npm ci` + `vitest --coverage`.

Sharding does not change that, and is not meant to — the cost here is CI
minutes, not human serialization. Narrowing those triggers is worth doing and is
a different piece of work.

---

## 6. Checklist for migrating the next document

HL21 §8, with the prose-specific steps substituted:

1. Confirm the split level. The hot spot is where entries are *added*; everything
   above the first heading at that level becomes `_meta.md` and must be the part
   nobody appends to.
2. Confirm no heading at that level lives inside a fenced code block. **Check; do
   not assume.**
3. Decide whether the document is newest-first. Set `newestFirst` accordingly —
   and expect prose ledgers to be newest-first, which is the case HL21's data
   ledgers never had.
4. Grep for programmatic reads of the monolith, including Vite globs and CI path
   filters. Route them through the loader, or keep the monolith as a generated
   artifact with a `--check` and **say why** — the reason will probably not be
   HL21's reason.
5. Check that adding a `<name>.d/` directory does not change the meaning of any
   directory scan that will now see it.
6. Add a `DocShardPlan` entry, run `--shard`, and assert the round trip is
   byte-identical **against the real document**, not only a fixture.
7. Land it when the file is quiet. Reshaping a file underneath an open branch
   breaks that branch.

---

## 7. Language Ladder changelog extension

Issue #13211 adds a third measured conflict point to this convention:

```text
code/programs/typescript/language-ladder/CHANGELOG.d/
```

The pre-migration file was touched by 34 commits after PR #12968. It is pure
prose: no runtime, package, or release script reads the tracked monolith. Its
shards are therefore the committed source of truth and `CHANGELOG.md` is an
ignored local rendered view, matching the current BACKLOG and data-package
changelog policy.

This document splits at level 2 and is newest-first. The recent history had 23
top-level entries accidentally written as `### Added` beneath the preceding
`## Unreleased` entry. The migration promotes only those entry headings to
level 2 before sharding; their body bytes and ordering do not change. Historical
`###` subsections under version headings stay inside their owning version shard.

The existing document sharder, real-document round-trip tests, append-only
deletion guard, ignored-monolith guard, and human-language path detector all
extend to this third plan. New Language Ladder entries are one uniquely named
level-2 shard above the current maximum rank. Future glyph changes edit that
fragment plus their script-owned evidence module and do not edit either shared
aggregate.
