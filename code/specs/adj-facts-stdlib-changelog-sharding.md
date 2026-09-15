# Sharding `adj-facts-stdlib`'s Changelog

## Goal

Third application of the mechanism in
[`lang-aot-changelog-sharding.md`](lang-aot-changelog-sharding.md), and the
first to use the bullet entry shape from
[`doc-shard-bullet-entries.md`](doc-shard-bullet-entries.md).

`code/specs/data/adj-facts-stdlib/CHANGELOG.md` is the repo's **worst remaining
live conflict generator**: 62 close touches of 110 over 21 days, top of the
ranking now that `lang-aot` and `algol-iir-compiler` are sharded.

## Every PR prepends to the same line

The clearest evidence in this whole effort, and it is not a proxy. The last
eight commits to this file:

| commit | lines added | first hunk at |
|---|---:|---:|
| `a146ad8f` | 103 | **line 8** |
| `664dd83f` | 37 | **line 8** |
| `415dc755` | 66 | **line 8** |
| `106b0265` | 61 | **line 8** |
| `615cbdaa` | 62 | **line 8** |
| `3a8ba3c1` | 66 | **line 8** |
| `33660876` | 61 | **line 8** |
| `56b2679d` | 68 | **line 8** |

Eight of eight, at the identical line, in a 10,541-line file. `## Unreleased`
is line 6; every author inserts directly beneath it. Two such PRs open
concurrently cannot merge cleanly, and this file sees several a day.

That also settles `newestFirst: true` by observation rather than by copying it
from the neighbouring plans.

## Model

One row, using the entry shape added for exactly this document:

```ts
{
  path: "code/specs/data/adj-facts-stdlib/CHANGELOG.md",
  headingLevel: 2,
  newestFirst: true,
  entryShape: "bullet",
}
```

The file has exactly **one** `##` heading over **323** top-level `- ` entries,
so heading-splitting yields a single shard holding the whole document.
`headingLevel` is not consulted under `"bullet"`; the preamble is the 161 bytes
above the first entry, carrying the `# Changelog`, its explanatory line, and
`## Unreleased`.

## What this does and does not buy

Unlike `algol-iir-compiler` — where `src/lib.rs`, `Cargo.toml` and
`CHANGELOG.md` scored identically, so sharding removed one conflict surface of
three — this document is **spec/content data with no sibling code file** that
every PR must also touch. The neighbouring `README.md` scores 22 against this
file's 62, so the changelog is the hot spot on its own.

Prepending an entry renames nothing: the new shard takes `max + stride`. A
reflow of an existing entry's first line renames that one shard, because in
bullet mode the digest covers a prose line rather than a heading; that
trade-off is documented on `entryShape` and is the reason the hot path was
measured (0 renames on a prepend, 324 → 325 shards).

## Scope

One registry row, one shard directory, one `.gitignore` line, and one entry in
each of `doc_shard_globs` and `tracked_doc_monoliths`. The tests added by
#15215 pin both guard lists against `DOC_SHARD_PLANS`, so omitting either fails
a test rather than shipping unguarded history — which is how the two omissions
in the `algol-iir` change were caught before it was committed.
