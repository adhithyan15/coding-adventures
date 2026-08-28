# HL30 — Sharded book-generation ownership

**Status:** specification, 2026-08-27

**Extends:** HL21's deterministic shard ordering and filesystem trust rules,
HL25's shard-native, no-aggregate storage rule, and HL29's stable generated
chapter identities. Tracks #13336, the parent contention program #13193, and
the broader human-language program #12206.

## 1. Outcome

Two agents registering different chapters of the same language must not edit a
shared language slice. `core/book-generation.d/` therefore owns each chapter,
backmatter declaration, and script set independently:

```text
core/book-generation.d/
  _meta.json
  targets.d/
    spanish-0001.json
  handwritten.d/
    spanish-0002.json
  reference-appendices.d/
    chinese-appendix-pronunciation.json
  glossaries.d/
    spanish-appendix-glossary.json
  answer-keys.d/
    spanish-appendix-answer-key.json
  indexes.d/
    spanish-appendix-index.json
  script-sets.d/
    0010-telugu-comparisons.json
```

This is an ownership migration, not a book-content or schema change. Folding
the canonical owners must preserve the former `book-generation.json` bytes,
top-level key order, array order, record key order, and public generator
behavior exactly. Neither a flat aggregate nor a per-language aggregate is
tracked or generated.

`_meta.json` is the only schema-level shared owner. It contains exactly
`version` and `sourceBaseUrl`; it does not contain any declaration array or
`scriptSets`. Daily chapter and book-structure work therefore does not touch it.

## 2. Stable owners and canonical order

`targets.d/` and `handwritten.d/` each store one record value per file. The
filename is `<language>-<NNNN>.json`, where `NNNN` is the record's positive
integer `chapter` written as four zero-padded ASCII digits. Both fields inside
the record must agree with the filename. This is the same `(language, chapter)`
identity used by HL29's generated hash owners.

The four backmatter sections also store one direct record value per file. Their
filename is `<language>-<output-basename>.json`: `output-basename` is the final
path component of `output` with its `.tex` suffix removed. Both the language and
the complete output path are checked against the filename. A section cannot
contain two records with the same resulting identity.

`scriptSets` is an object in the reconstructed document, so its key is recovered
from the filename rather than repeated in the shard value. Each shard contains
the raw script-entry array and is named `<OOOO>-<id>.json`. `OOOO` is a positive
four-digit ordinal spaced by ten (`0010`, `0020`, ...); `id` is the script-set
key. The ordinal preserves the authored object-key order; later additions
append a new spaced ordinal without renaming existing owners.

Readers sort owner names by raw code unit, never filesystem enumeration order
or locale collation. That order reproduces the historical arrays: language
first, then padded chapter or output basename. Section directories are folded
in the fixed historical top-level order
`referenceAppendices`, `glossaries`, `answerKeys`, `indexes`, `targets`, and
`handwritten`; `scriptSets` is reconstructed at its historical position after
`sourceBaseUrl` and before those arrays. Shards are serialized as canonical
two-space-indented JSON with one trailing newline, preserving record key order.

## 3. Exact cross-ledger identity gates

The chapter shard set is complete only when four separately reported identities
agree:

1. the languages present across book-generation owners are exactly the active
   `core/languages.json` registry, with no unregistered or missing language;
2. `targets.d/` identities are exactly the
   `core/generated-book-hashes/<language>.d/NNNN.json` identities;
3. the union of `targets.d/` and `handwritten.d/` identities is exactly the
   `core/generated-narration-hashes/<language>.d/NNNN.json` identities; and
4. that same union is exactly the `(language, chapter)` capability identities
   authored under each track's `chapters.d/`.

These are equality gates, not subset checks. A target/handwritten duplicate,
an undeclared generated hash, a chapter capability with no book declaration,
or a declaration with no capability fails closed. Title and label remain
derived from the capability ledger and are forbidden in book-generation
owners.

Backmatter has its own independent closure. Glossary, answer-key, and index
identities must be exactly the registry-derived standard output for every
language. Generated pronunciation-reference owners must exactly match the
generated reference artifacts (identified by their generated-file marker).
Every script set must be referenced by a surviving declaration, so deleting a
script-set owner also fails the fold rather than shrinking it silently.

The checks compare identities in memory. They must not introduce a mutable
corpus-wide hash or expected-count file that recreates a shared edit point.

## 4. Filesystem trust and mutation

The canonical root accepts exactly `_meta.json` and the seven named real
section directories. Every section accepts only its canonical direct-child
owner grammar. Enumeration inspects every directory entry before parsing; it
must not filter an unexpected name, nesting level, file type, or compatibility
aggregate out of sight.

Read, check, migration, and in-memory reconstruction paths reject:

- missing, stale, unexpected, malformed, duplicate, or mismatched owners;
- the former `<language>.json` grouped shards or a resurrected
  `core/book-generation.json` monolith;
- symlinked roots, section directories, owner files, or ancestor components;
- non-regular files and unexpected nested directories;
- absolute, traversal-shaped, separator-bearing, dot-segment, reserved-device,
  or otherwise non-canonical identities and output paths;
- dangerous object keys such as prototype-mutating names; and
- metadata in section owners or section data in `_meta.json`.

Containment is checked component by component beneath the fixed curriculum
root before any read or write. Parse errors identify the owner without echoing
hostile file bytes. Check mode is read-only: it never repairs, deletes, or
silently ignores an owner. Migration mode validates the source and complete
projected owner set before replacement, uses exclusive file creation so a
concurrent or symlink collision fails, and removes only the precisely
identified legacy owners.

## 5. Shard tooling and consumers

`loadBookGenerationConfig` folds the owner tree directly and returns the same
public configuration shape consumed by book generation, narration generation,
progress reporting, handwritten parity, and `book.tex` generation. Callers do
not reconstruct a second projection or read a compatibility aggregate.

One migration/test-only compatibility path remains: when
`core/book-generation.d/` is entirely absent, the loader may read a legacy
monolith fixture. The presence of any owner directory commits the read to the
canonical projection. A malformed, hostile, or incomplete `.d/` tree fails
closed and must never fall back to a nearby monolith. In the repository,
`check:shards` requires the complete owner tree and requires the monolith to be
absent.

`npm run check:shards` validates both the structural owner contract and the
byte-exact reconstructed document. `npm run shard -- core/book-generation.json`
is migration/import behavior only; steady-state authoring edits the owning
file. The ordinary `unshard` command refuses this removed-monolith ledger,
because its output would immediately violate `check:shards`. An explicit audit
may call the pure in-memory reconstruction helper; it must not write a tracked
or accepted compatibility input.

The capability manifest continues to authorize reads of the curriculum tree.
This migration adds no generator write target: book-generation owners are
authored configuration, while generated book, narration, hash, and progress
outputs retain their existing narrowly declared write capabilities.

## 6. Migration proof

Immediately before removing the 23 grouped language files, fold fresh `main`
and compare the result byte for byte with the pre-migration document. The
reproducible baseline is:

| Section | Records |
| --- | ---: |
| `referenceAppendices` | 6 |
| `glossaries` | 23 |
| `answerKeys` | 23 |
| `indexes` | 23 |
| `targets` | 1,088 |
| `handwritten` | 69 |
| `scriptSets` | 8 |

The canonical reconstructed document is 188,438 UTF-8 bytes and has SHA-256
`960826bab96d7cf2c30cd6a5d0287cfa83b13170808a3e13671469548628ad07`.
This exact file digest is one-time migration evidence, not a steady-state pin
that future owners must update.

Fresh `main` must be merged immediately before migration so an in-flight
chapter registration cannot be omitted behind a mechanically valid old
baseline. After migration, run the complete Human Language Data suite,
`check:shards`, all book/narration/hash/progress checks, and the all-books
compile validation.

## 7. Acceptance

1. one chapter registration touches one stable chapter owner, not a language or
   corpus aggregate;
2. backmatter and script-set changes touch independently owned stable files;
3. the reconstructed 188,438-byte document matches the fresh-main baseline
   exactly, including every record and ordering decision;
4. registry, generated-book, generated-narration, capability, and backmatter
   identity sets pass their exact equality gates;
5. malformed, missing, stale, unexpected, duplicate, symlinked, non-regular,
   mismatched, dangerous, nested, and escaping owners fail closed;
6. every consumer reads the canonical owner tree and no flat or per-language
   compatibility aggregate remains; and
7. `_meta.json` is the only schema-level shared owner, and ordinary chapter or
   backmatter authoring never changes it.
