## HL-C408 — `practises.knowledge` accepts an atom id that does not exist

**Status: OPEN.** Found after writing a non-existent atom id into a Malayalam
lesson for the **third** time in one session.

**`curriculum.ts` checks one of the two atom lists by name and not the other.**

| field | check | message |
|---|---|---|
| `requires.knowledge` | **yes** — `curriculum.ts:747` | *required atom 'X' is not introduced by a transitive prerequisite* |
| `practises.knowledge` | **no** | — |

An id in `practises` is only ever compared against the lesson's **own** body
blocks (`curriculum.ts:862`, *"practised atom 'X' is not assessed by any body
block"*). A fabricated id therefore produces a message that reads as a missing
`assesses=[...]` entry, and adding the fabricated id to a block silences it.
Nothing downstream — not the twelve gates, not the full suite — looks at whether
the atom exists.

**The three ids that got through, all well-formed under the corpus's dominant
`ML-<KIND>-C<chapter>-<TOPIC>-<nn>` shape:**

| written | actual |
|---|---|
| `ML-LEX-C13-BODY-01` | `ML-CONCEPT-C13-SHAREERA-BHAAGANGAL-01` |
| `ML-CONCEPT-C02-NII-01` | `ML-LEX-NII-NINGAL-01` |
| `ML-CONCEPT-C03-SUKHAM-01` | `ML-LEX-SUKHAM-01` |

All three cite **early** chapters, whose atoms predate the naming convention and
carry no chapter segment. That is not a coincidence: the ids most likely to be
reconstructed by pattern are exactly the early anchors that a review or recall
lesson reaches for.

**What closing this needs.** Give `practises.knowledge` (and `reviews_of`'s
atoms, if they are resolved anywhere) an existence check against the set of all
`introduces` ids across the track, with a message that names the id as unknown
rather than as unassessed. Order matters: the unknown-atom check must run
**before** the not-assessed check, or the fabricated id still reports as a block
problem.

**The existing debt was measured when this shard was filed, and it is zero.**
Across all 23 tracks, **8810** distinct ids appear in a `practises` list and
**every one** of them is introduced somewhere. So this is a hole that has not yet
been fallen into: the not-assessed message has, so far, always been resolved by
correcting the id rather than by adding the fabricated id to a block. That is
luck plus review, not a guarantee, and it is why this is worth closing before it
costs something. Re-run the sweep when closing — the number belongs in the
closing note.

```sh
# dangling = every id in any practises list, minus every id in any introduces
```
