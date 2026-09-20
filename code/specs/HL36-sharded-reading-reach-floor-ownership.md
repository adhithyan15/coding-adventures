# HL36 — Sharded Reading-Reach Floor Ownership

The reading-reach floor is authored policy, not a generated snapshot. It keeps
each language/level's already-demonstrated reading reach from falling when a
passage is shortened or removed. The old `core/reading-reach-floor.json` put
every independent track in one conflict domain.

## Canonical owners

`core/reading-reach-floor.d/_meta.json` owns the schema version and stable
ratchet explanation. Every task-shape inventory owns exactly one direct child:

```text
core/reading-reach-floor.d/<language>--<lowercase-level>.json
```

Its canonical body is `{ language, level, floor }`. The filename repeats the
body identity. Floors are non-negative integers. An inventory whose reading has
not reached a published part has an explicit zero owner, so the expected owner
set comes independently from task-shape inventories instead of from whichever
ratchet owners survived.

The historical public shape remains `{ version, about, floors }`. Folding omits
zero owners, preserving the rule that an absent public entry floors at zero and
preserving all existing positive identities and values.

## Failure rules

Readers reject missing or unexpected owners before parsing their bodies. They
also reject duplicate identities, case-fold collisions, unsafe language or
level paths, nesting, symbolic links, non-regular files, filename/body
mismatches, malformed floors, noncanonical bytes, and a resurrected aggregate.
The floor comparison remains `measured >= floor`; a red ratchet is repaired by
restoring reading evidence, never by lowering policy.

## Parallel authoring

When a passage makes more exam parts reachable, edit only that language/level
owner. Adding a task-shape inventory adds its own zero owner. Neither operation
requires touching another track or a regenerated aggregate.
