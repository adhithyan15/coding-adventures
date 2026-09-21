## HL-C415-58db71cc — `ML-A1-F-33`'s note denies a pronoun the track teaches

**Status: OPEN.** Found while closing `ML-A1-LEX-41` for chapter 112, which was
itself a stale-note find. This is the same defect in the same inventory.

`core/exam-inventory-malayalam-a1.d/1560-ML-A1-F-33.json` reads:

> Requires undu with a name and a question particle. Both parts are taught, the
> assembly is not, and **there is no third-person pronoun (ML-A1-PRON-03)** to
> refer to the person once found.

The clause after the second comma is false, and its own citation disproves it.

| claim | actual |
|---|---|
| no third-person pronoun | `ML-C71-avan-aval` (seq 2430) teaches **അവൻ / അവൾ**, concept_tag `ML-THIRD-PERSON-FAMILIAR`, atom `ML-LEX-C71-AVAN-AVAL-01` |
| — | `ML-C87-avane` (seq 3060) teaches **അവനെ**, the object form |
| `ML-A1-PRON-03` is open | `0220-ML-A1-PRON-03.json` is **CLOSED**, probe `["ML-LEX-C71-AVAN-AVAL-01", "ML-LEX-C71-AVAR-01"]` |

So the note names a sibling point as evidence of a gap, and that sibling closed.
`PRON-03`'s own note records that `avan`/`aval`/`avar` once appeared in ZERO
files — which is what F-33's author saw, and which chapter 71 then fixed without
F-33 being revisited.

**What is still true:** the ASSEMBLY half. `undu` + a name + a question particle
is not taught as one move. Whoever closes F-33 should keep that half and delete
the pronoun clause, rather than reading the whole note as discredited.

**Why this keeps happening.** A note is written when a point is surveyed and is
never re-read when a NEIGHBOURING point closes. Four of the last five Malayalam
points closed turned on a note that had gone stale this way, and the surrounding
mechanical work produced no defects at all. A note that cites another point by
id is the highest-risk kind, because the id makes it look checkable while
nothing checks it. A cheap guard would be a test asserting that no open point's
note names a `ML-A1-*` id that is itself covered.
