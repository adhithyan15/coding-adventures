# HL32 — Sharded lesson-modality ownership

**Status:** specification, 2026-08-28

**Extends:** HL08's derived voice/sight/pen contract, HL21's deterministic
filesystem rules, and HL25's shard-native, no-aggregate storage rule. Tracks
#13375, parent #13193, and program #12206.

## 1. Outcome

Changing two lessons in one language must not rewrite one generated language
aggregate. Lesson modality therefore uses stable direct owners:

```text
core/lesson-modality/
  arabic.d/
    _meta.json
    <lesson-id>.json
  ...
  urdu.d/
    _meta.json
    <lesson-id>.json
```

Each registered language has exactly one `<language>.d/` directory. Its
`_meta.json` owns only stable `version`, `language`, `algorithm`, `features`, and
`policy` fields. Every other direct child is `<lesson-id>.json` and contains
exactly `{ "lesson": ..., "findings": [...] }` for that canonical lesson.

Fresh main has 23 languages and 4,462 lessons, so the canonical tree has 4,485
files: 23 metadata owners plus 4,462 lesson owners. The old 23
`core/lesson-modality/<language>.json` aggregates are neither tracked nor emitted.

## 2. Exact identity closure

Completeness is established before owner contents can define the corpus:

1. `core/languages.json` supplies the exact 23-language directory set.
2. Parsed canonical lesson Markdown supplies the exact lesson-id set and language
   assignment for every directory.
3. Generated narration-hash chapter owners independently supply the exact lesson-id
   set and language assignment.

All three projections must agree exactly. Subset checks are insufficient: they let a
cleanly deleted lesson owner or a stale generated owner survive undetected. The
reader checks directory and direct-child types, names, and case-fold uniqueness and
compares these independent identity sets before it opens lesson owner bytes.

Each opened file passes the guarded ledger boundary. Owners must be canonical
two-space JSON with one trailing newline, contain exact safe keys, and be direct
regular files rather than directories, symbolic links, or other special entries.
The directory identity, filename stem, `lesson.id`, and `lesson.language` must agree.
Unsafe or traversal-shaped ids, dangerous object keys, duplicates, case-fold
collisions, unexpected entries, nesting, and noncanonical bytes fail closed.

## 3. Reconstruction without aggregates

Canonical readers consume only the `.d/` owner tree. A flat
`core/lesson-modality/<language>.json` file is an error even when its contents happen
to match, and there is no legacy fallback.

The fold preserves the public `loadModalityManifest()` result while deriving every
high-level projection from direct owners: language and corpus source hashes, lesson
ordering, summary counts, track rows, chapter rollups, drivable prefixes, and the
corpus summary. These values remain useful API output but are not tracked shared
files. Language Ladder does not import this corpus, so the migration does not add a
browser module or one browser loader per lesson.

`check:modality` verifies exact filenames, exact identity sets, canonical bytes, and
the reconstructed result against the current lesson derivation. Clean deletion,
stale owners, aggregate resurrection, and byte drift all fail the gate.

## 4. Generated migration

`generate:modality` is a staged replacement, not an in-place partial rewrite. It
builds all 4,485 expected owners beneath a private temporary root, validates direct
entry types, canonical bytes, identity closure, and the fully reconstructed public
manifest there, then renames each complete language tree into its canonical place.
Legacy aggregates are removed only after their validated replacements exist.

An unsafe or incomplete destination leaves the prior aggregate untouched. Once
canonical owners exist, generation refuses to use a resurrected aggregate as source
or overwrite the owner tree from it. No `--unshard` or compatibility output may
recreate the removed per-language files.

The package capability manifest grants writes only to
`core/lesson-modality/*.d/*.json`. It deliberately does not grant the flat
`core/lesson-modality/*.json` path, so the filesystem policy enforces the same
no-aggregate boundary as the loader and checker.
