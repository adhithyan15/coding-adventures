# HL39 — Single-owner curriculum spine membership

**Status:** specification, 2026-09-20

**Extends:** HL21. Tracks #15669 and parent #13193.

## Decision

Each curriculum path shard owns its spine membership exactly once:

```json
{
  "id": "ML-PATH-110-MESSAGE",
  "spine_node": "SPINE-DEFINITE-REFERENCE"
}
```

`curriculum.d/spine/*.json` stores only independently authored omission and
relocation policy. It must not store `segments`. The loader walks path shards in
their authored filename order and derives every public
`curriculum.spine[node].segments` array from those `spine_node` edges.

Adding a path segment therefore creates its new path owner and does not edit an
existing spine owner. This applies to every registered language.

## Compatibility and gates

The public `LanguageCurriculum` shape is unchanged. Before removing the 920
duplicate arrays, the canonical JSON for all 23 loaded curricula hashed to
`ef4c97754b1f7ca4ff6feaa22cd98ad66c6415613fde9d8184ef2f783fdcc7ae`.
After deriving reverse membership from path owners it hashes to the same value.

The read boundary fails closed when a stored `segments` copy returns, a path id
is duplicated, or a path names a spine owner that does not exist. Existing
curriculum validation continues to reject duplicate lessons, unknown shared
spine nodes, missing spine policy owners, omission/relocation drift, unmapped
lessons, and prerequisite cycles or ordering defects. `check:shards` still
checks owner filenames, identities, case-fold collisions, canonical bytes, and
aggregate resurrection.

## Authoring rule

To attach a new segment to a spine node, set `spine_node` in the new path shard.
Do not edit the node's spine policy owner unless its authored `omits` or
`relocates` policy genuinely changes.
