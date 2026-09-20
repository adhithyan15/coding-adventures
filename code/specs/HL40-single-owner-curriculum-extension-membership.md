# HL40 — Single-owner curriculum extension membership

**Status:** specification, 2026-09-20

**Extends:** HL21 and HL39. Tracks #15729 and parent #13193.

## Decision

The path shard owns both the exact lesson order and each unambiguous extension
membership edge. Extension lessons are tagged in that ordered walk:

```json
{
  "lessons": [
    { "lesson": "HI-S135-letter-au", "extension": "HI-EXT-100-SCRIPT-RECOGNITION" },
    "HI-W12-schwa-drop",
    { "lesson": "HI-S136-letter-gha", "extension": "HI-EXT-100-SCRIPT-RECOGNITION" }
  ]
}
```

Plain strings remain path-core lessons. An extension owner with tagged path
membership omits `lessons`; the shard reader restores both public string arrays
in path order. This preserves interleaving without guessing from lesson ids,
categories, prefixes, or placement labels.

The migration applies only when every extension lesson occurs exactly once in
the attached path walk, in the same order, and no occurrence matches two
attached extensions. Partial, repeated, shared, or otherwise ambiguous shapes
retain their explicit extension arrays.

## Compatibility and failure boundary

The public `LanguageCurriculum` interfaces remain unchanged: path and extension
`lessons` are still `string[]`. The corpus-wide shard round-trip and curriculum
validation gates prove the reconstructed order and membership. Before and
after migration, canonical JSON for all 23 loaded curricula hashes to
`ef4c97754b1f7ca4ff6feaa22cd98ad66c6415613fde9d8184ef2f783fdcc7ae`.

The read boundary rejects unknown or unattached extension tags, duplicate
derived lessons, malformed tagged entries, and dual ownership where a tagged
path points at an extension that still stores `lessons`. Existing validation
continues to reject missing owners, duplicate public lessons, bad placement,
unknown lessons, and prerequisite/order defects.

## Authoring rule

Add a new extension-owned lesson once, at its prerequisite-safe position in the
path shard, and tag it with the attached extension id. Do not edit that
extension's derived reverse list. Add path-core lessons as plain strings.
