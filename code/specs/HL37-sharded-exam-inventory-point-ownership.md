# HL37 — Sharded Exam-Inventory Point Ownership

Exam inventories are authored external or editorial targets. Their metadata,
scope, provenance, and points remain one public `ExamInventory`, but the source
need not be one conflict domain. Malayalam A1 crossed that threshold at 4,839
lines and 39 touches in the latest 200 human-language commits; Hindi A1 followed
at 2,881 lines and 22 touches after the same boundary proved stable.

## Canonical layout

Each migrated A1 source is:

```text
core/exam-inventory-<language>-a1.d/_meta.json
core/exam-inventory-<language>-a1.d/NNNN-<POINT-ID>.json
```

Metadata owns every top-level field except `points`, plus `pointIds`: the exact
ordered identity manifest. Each point owner contains the complete point and
repeats its id. Four-digit ordinals preserve source order while leaving gaps
for later insertion. Folding removes `pointIds`, restores `points` in manifest
order, and produces the historical public object byte-for-byte.

The identity manifest is stable inventory scope, not a generated aggregate.
Changing a probe, note, label, anchor, or derivation edits only that point's
owner. Adding or removing the externally/editorially enumerated target itself
is the rarer scope change that also updates metadata.

## Failure rules

Once the owner directory exists it is the only source of truth. Readers reject
a sibling aggregate, missing or extra points, duplicate or case-fold-colliding
identities, order drift, unsafe ids, filename/body mismatches, invalid ordinals,
noncanonical bytes, nesting, symbolic links, and non-regular owners. The normal
exam-inventory validation still runs after reconstruction, so scope, probes,
reserved categories, and duplicate ids retain their existing semantics.

## Parallel authoring

A chapter agent edits only the point files whose evidence changed. New work
must not recreate a retired sibling `core/exam-inventory-<language>-a1.json` or
copy point history into a shared test. Other languages remain compatible
monoliths until their own measured contention justifies the same migration.
