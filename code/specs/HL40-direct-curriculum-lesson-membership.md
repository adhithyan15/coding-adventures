# HL40: Direct curriculum lesson membership

## Status

Accepted and implemented for every registered human-language track.

## Problem

Each lesson id used to be stored in two aggregate places:

1. one `curriculum.d/path/*.json` shard's `lessons` array; and
2. usually one `curriculum.d/extensions/*.json` shard's `lessons` array.

Eleven otherwise independent Hindi glyph changes edited the same path and
extension owners. Across the complete corpus, 7,196 lesson files produced 7,196
path memberships and 6,543 extension memberships. All 7,196 lessons belonged to
exactly one path segment, but the repeated aggregate arrays made independent
lesson work collide and allowed the two declarations to drift.

## Canonical ownership

Every lesson has one direct owner:

```text
<track>/curriculum-membership.d/<lesson-id>.json
```

The owner contains exactly:

```json
{
  "id": "HI-S136-letter-gha",
  "pathSegment": "HI-PATH-100",
  "pathOrder": 44,
  "extensions": [
    { "id": "HI-EXT-100-SCRIPT-RECOGNITION", "order": 33 }
  ]
}
```

`_meta.json` owns only stable format and language identity. It deliberately
does not list lesson ids: such a manifest would become the next shared append
point. The independent completeness source is the set of canonical
`lessons/<lesson-id>.md` filenames.

Path owners retain path identity, the shared spine edge, and extension
placement. Extension owners retain stage, kind, category, can-do, and
prerequisites. Neither may store `lessons`.

## Reconstruction

The loader folds direct owners into the historical public shape:

- `pathSegment` selects exactly one path;
- `pathOrder` reconstructs that path's `lessons` array;
- each extension `{ id, order }` reconstructs one extension's `lessons` array.

Orders are non-negative, unique, and dense from zero. They are explicit because
24 path segments and 13 extensions intentionally differ from global lesson
`sequence` order. An extension declared by a lesson must be attached to that
lesson's path in `before`, `inline`, or `after`.

The migration preserves the exact public curriculum graph. SHA-256 over
`JSON.stringify(loadLanguageCurricula())` remains:

```text
ef4c97754b1f7ca4ff6feaa22cd98ad66c6415613fde9d8184ef2f783fdcc7ae
```

Language Ladder uses the same strict owner fold in its bounded build-time
virtual modules, so browser and Node consumers receive the same projection.

## Integrity and safety

The reader rejects:

- a missing, extra, duplicate, non-file, or case-fold-colliding owner;
- a filename/body lesson-id mismatch;
- noncanonical JSON bytes or unexpected fields;
- a symlinked owner directory, lesson directory, or direct child;
- unsafe language or lesson ids;
- a missing path or extension target;
- duplicate or non-dense orders;
- an extension edge not attached to the selected path; and
- resurrection of a path or extension `lessons` array.

## Authoring rule

Adding or moving one lesson edits that lesson's direct membership owner. A new
path or extension still adds its independent definition owner, but ordinary
lesson growth never edits an existing path or extension membership array.
