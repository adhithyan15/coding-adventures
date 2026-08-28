# HL29 — Sharded generated chapter-hash ownership

**Status:** specification, 2026-08-27

**Extends:** HL21's shard ordering and filesystem trust rules and HL25's
shard-native, no-aggregate storage rule. Tracks #13334, the parent contention
program #13193, the broader human-language program #12206, and the derived
consumer work in #13301.

## 1. Outcome

Two agents regenerating different chapters of the same language must not edit
the same generated book-hash or narration-hash manifest. The smallest tracked
owner is therefore one chapter, not one language:

```text
core/generated-book-hashes/<language>.d/
  _meta.json
  0001.json
  0002.json

core/generated-narration-hashes/<language>.d/
  _meta.json
  0001.json
  0002.json
```

This is an ownership migration, not a content or hashing change. It preserves
every chapter record, source and output fingerprint, lesson id and order,
narration-width setting, generated-artifact path, finding, and consumer-visible
status.

## 2. Stable chapter identities

Each language owns exactly one real `<language>.d/` directory. `_meta.json`
owns the document-wide fields: `version`, `language`, and `algorithm` for both
ledger families, plus `maxLinearisableTableColumns` for narration. It must not
contain a `chapters` array.

Each other file owns exactly one chapter record and is named from the record's
positive integer `chapter` as four zero-padded ASCII digits: chapter 1 is
`0001.json`. The directory name is the language identity and the filename is
the chapter identity; both must agree with the record. Adding or updating one
chapter does not rename its neighbours or touch `_meta.json`. Narration
findings live with the chapter that produced them rather than in a shared
language-level array.

Readers sort filenames by raw code unit, never filesystem enumeration order or
locale collation. Four-digit numeric identity therefore reproduces ascending
chapter order deterministically. Duplicate language owners, duplicate chapter
numbers, unexpected identities, and a filename/record mismatch fail closed.

The former `<language>.json` files are forbidden once migrated. No generated or
compatibility aggregate is tracked beside the canonical directories, because
such an aggregate would remain the shared file this migration removes.

## 3. Generator and check behavior

`generate:books` and `generate:narration` write the metadata and chapter owners
directly. They do not build a language aggregate as an intermediate tracked
artifact. An ordinary source change should leave a diff only in the generated
artifact and the one chapter-hash owner affected by that source.

The read-only checks derive the complete expected owner set in memory and
compare each expected file byte for byte. They reject:

- a missing, stale, unexpected, malformed, or non-regular owner;
- a flat per-language aggregate or an owner outside its language directory;
- a symlinked language directory, metadata file, or chapter file;
- a traversal-shaped or non-canonical language or chapter identity;
- duplicate directory, record-language, or record-chapter ownership; and
- metadata in a chapter file or chapter data in `_meta.json`.

Before writing, tools resolve and re-check containment beneath the fixed ledger
root. Enumeration must inspect every directory entry rather than filtering
malformed names out of sight. Parse errors are scrubbed so hostile file bytes do
not leak into CI logs. The check path must never repair, delete, or silently
ignore an unexpected owner.

## 4. Filesystem consumers

Progress generation discovers the registered language directories and folds
their chapter owners in deterministic order. It consumes the canonical shard
sets directly and does not require a regenerated language-level manifest.

Filesystem capability declarations name the new `*.d/*.json` write targets.
Broad curriculum read access does not authorize a generator to write a flat
aggregate or escape its generated-hash roots.

## 5. Bounded browser boundary

Language Ladder must not replace 23 language JSON loaders with one browser
module key per chapter. That would turn 1,088 current book records—and every
future C2 chapter—into growing import metadata and potentially one request or
chunk per owner.

Instead, the existing build-time human-language ledger plugin exposes exactly
one lazy book-hash loader per registered language: 23 loaders at migration
time. Loading one virtual language module folds that language's `_meta.json`
and `NNNN.json` owners during the build. Chapter count may grow without growing
the public loader table.

Every contributing file is registered with the Vite watch graph. An added or
removed owner invalidates the corresponding virtual module and triggers the
same explicit development reload used for other sharded ledgers. A failed lazy
load remains non-fatal and observable: book status honestly degrades to
`not-generated`, the error is reported, and the rest of the app remains usable.
The book-ledgers chunk stays lazy and within its existing bundle budget.

## 6. Migration proof

Before deleting the 46 flat language manifests, assemble both new corpora and
compare them with fresh `main`. The exact baselines are:

| Ledger    | Records | Ordered semantic SHA-256                                           | Ordered identity SHA-256                                           |
| --------- | ------: | ------------------------------------------------------------------ | ------------------------------------------------------------------ |
| Book      |   1,088 | `3e2e84ed6fc7ef933958c5a4c8c48be6ef49851079856ec8e8286b3018a8fd04` | `a790650e10b504ff09885da636876adbeafb549340290b67d56f2494c1c77bef` |
| Narration |   1,157 | `8038f2e6efef44a3e17e9ac6e43801643c2fc56bcc49696ec6c8f280f20482f8` | `824093fb15a630300aa37027c4c84babc0813274967ef5d15e0aac9cfd3adeb9` |

The baselines are computed by raw-code-unit sorting the 23 flat manifest
filenames, concatenating each parsed manifest's `chapters` array in its stored
order, and taking SHA-256 over `JSON.stringify(chapters)`. The identity digest
uses the same records and hashes
`JSON.stringify(chapters.map(({ language, chapter }) =>
`${language}:${chapter}`))`. This exact serialization is intentionally stated:
it is reproducible from fresh `main` rather than a tool- or whitespace-specific
file digest. The corpus-wide values are one-time migration pins, not a mutable
steady-state file that every future chapter edit must update.

Fresh `main` must be merged immediately before migration so an in-flight
generated-artifact PR cannot be lost behind a mechanically correct old
baseline. After migration, run the complete Human Language Data and Language
Ladder suites, all generated book/narration checks, the all-books validation,
and the Language Ladder production bundle gate.

## 7. Acceptance

1. one generated chapter update touches one stable `NNNN.json` owner, not a
   per-language or corpus aggregate;
2. all 1,088 book and 1,157 narration records match the fresh-main baselines;
3. malformed, missing, stale, unexpected, duplicate, symlinked, non-regular,
   mismatched, and escaping owners fail closed;
4. generation and progress read/write the canonical directories directly;
5. Language Ladder exposes exactly 23 lazy language loaders, preserves its
   load-failure behavior, and does not regress the eager or book-ledger bundle;
6. no flat hash manifest or other tracked aggregate remains; and
7. the migration lands before lesson-modality sharding and before broad C2
   curriculum rewriting begins.
