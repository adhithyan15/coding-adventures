---
category: Testing & coverage
---

# An atom id that follows the corpus's dominant naming pattern is still an invented id

Three times in one session I wrote a knowledge-atom id into a Malayalam lesson's
`practises.knowledge` that does not exist:

| written | actual |
|---|---|
| `ML-LEX-C13-BODY-01` | `ML-CONCEPT-C13-SHAREERA-BHAAGANGAL-01` |
| `ML-CONCEPT-C02-NII-01` | `ML-LEX-NII-NINGAL-01` |
| `ML-CONCEPT-C03-SUKHAM-01` | `ML-LEX-SUKHAM-01` |

Each invented id was *well-formed*. The corpus's dominant shape is
`ML-<KIND>-C<chapter>-<TOPIC>-<nn>`, so an id assembled from that shape reads as
correct to your own eye and survives every re-read of the draft. The early
chapters, though, predate the convention: their atoms carry no chapter segment
(`ML-LEX-SUKHAM-01`) and sometimes a different `KIND`. The ids you are most
likely to invent are therefore exactly the ones you reach for when citing an
*early* anchor — which is what a review or recall lesson does constantly.

**The validator's coverage is uneven, and the gap is exactly where I kept
writing.** An invented id in `requires.knowledge` is caught by name —
`curriculum.ts:747` reports *"required atom 'X' is not introduced by a transitive
prerequisite"*. An invented id in `practises.knowledge` has no such check. All
three of mine were in `practises`, and the only message that fires is about the
*block*, not the id:

```
ML-R107-medicine-recall: practised atom 'ML-CONCEPT-C03-SUKHAM-01'
is not assessed by any body block
```

That reads like a missing `assesses=[...]` entry, so the reflex fix is to add
the fabricated id to a block — which makes validate pass and leaves a dangling
reference behind. A corpus-wide sweep says that has not actually happened yet:
of **8810** distinct ids in `practises` lists across 23 tracks, all 8810 are
introduced somewhere. The hole is real and so far only review has kept anything
from falling into it (`HL-C408`).

**Do this instead.** Never type an anchor atom id from memory or by pattern.
Grep the lesson that introduces it and copy the string:

```sh
grep -n 'introduces=\[' code/learning/human-languages/<track>/lessons/<LESSON>.md
```

And when validate reports an atom "not assessed by any body block", confirm the
id exists somewhere as an `introduces` before touching any `assesses` list —
a non-existent atom and a genuinely unassessed one produce the same message:

```sh
grep -rn 'introduces=\[.*<THE-ID>' code/learning/human-languages/<track>/lessons/
```

The durable fix is to give `practises.knowledge` the existence check that
`requires.knowledge` already has; until that exists, the grep is the check.
