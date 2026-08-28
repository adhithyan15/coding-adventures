# HL31 — Sharded sound-tag ownership

**Status:** specification, 2026-08-27

**Extends:** HL21's deterministic ordering and filesystem trust rules and
HL25's shard-native, no-aggregate storage rule. Tracks #13338, parent #13193,
and program #12206.

## 1. Outcome

Pronunciation work in two languages must not edit one cross-language file. The
authored vocabulary accepted by lesson `sounds:` frontmatter therefore lives in
stable per-language owners:

```text
core/sound-tags.d/
  _meta.json
  arabic.json
  bengali.json
  ...
  urdu.json
```

`_meta.json` contains exactly `{ "version": 1 }`. Each language file contains
exactly `{ "language": <filename stem>, "tags": [...] }`. Tags retain the
historical lowercase hyphenated grammar and raw-code-unit sorted/unique order.
Empty language vocabularies remain valid. Tags may intentionally repeat across
languages; only duplicates within one owner are forbidden.

There is no tracked or generated `core/sound-tags.json`. Canonical consumers
fold the owner tree into the unchanged public `{ version, tracks }` shape.

## 2. Exact completeness before bytes

The expected owner filenames come from the independently authored
`core/languages.json` registry. A reader first enumerates every direct child,
rejects nesting, symbolic links, non-regular files, unexpected extensions,
unsafe names, and Windows device names, and compares the complete filename set
with the registry. Only after that equality succeeds may it open `_meta.json`
or a language owner.

This ordering closes clean deletion. If `tamil.json` disappears while another
surviving owner is malformed, the result is first and unambiguously “Tamil is
missing,” not a smaller corpus assembled from whatever files survived.

Each opened owner is read through the common guarded ledger boundary. It must
have exact keys, canonical two-space JSON bytes with one trailing newline, and
an embedded language equal to its filename. Dangerous keys, malformed JSON,
duplicates, unsorted tags, traversal-shaped names, case drift, and a resurrected
aggregate all fail closed.

## 3. Migration and preservation

The one-time source aggregate measured 27,912 bytes, 23 language tracks, and
1,251 `(language, tag)` identities. Its SHA-256 was
`281fd1fe87ef468124abde16fffed8f1d6607ea557d00905358374a3108d7588`.

`shard-cli --shard core/sound-tags.json` builds all 24 files under a private
staging root, validates their exact filename set and canonical bytes, folds
them, and requires byte-for-byte equality with the source registry. It then
renames the complete owner directory into place and removes the aggregate last.
An unsafe destination leaves the source aggregate untouched. `--unshard` is
refused because recreating the removed aggregate would restore the conflict and
create a file canonical readers intentionally reject.

The whole-corpus digest is migration evidence, not a steady-state shared test
expectation. Future legitimate changes edit only the owning language file;
structural equality, canonical bytes, local sorting, and independent registry
closure preserve safety without recreating one shared hash line.

## 4. Validation

`npm run check:shards -- core/sound-tags.json` strictly folds the owner tree,
checks exact registry identities and canonical bytes, and rejects aggregate
resurrection. The integration gate then validates every lesson `sounds:` tag
against that reconstructed closed vocabulary. `loadEverything()` passes the
same already-loaded language registry object into the sound-tag loader so both
returned projections come from one completeness snapshot.
