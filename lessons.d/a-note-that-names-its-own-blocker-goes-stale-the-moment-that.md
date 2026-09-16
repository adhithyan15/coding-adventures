---
category: Repo policy / workflow reminders
---

# A note that names its own blocker goes stale the moment that blocker closes

Hindi chapter 100 closed four exam points. **Two of them needed no new insight
at all** — they had been unblocked by earlier chapters in the same campaign, and
nothing had re-read them:

| point | what the note said | what had happened |
|---|---|---|
| `HI-A1-ADJ-09` | *"Hindi compares with the postposition `se`, which is untaught. See HI-A1-POST-04."* | `POST-04` closed in chapter 98 |
| `HI-A1-ADV-06` | *"No word for 'badly' — neither `bura` nor `kharab` — is introduced."* | *burā* was introduced in chapter 91 |

`ADJ-09` is the sharper case. The note **names the point that would unblock
it**, by id. When `POST-04` closed two chapters later, nothing followed the arrow
back, and a point that was ready to close sat uncovered while other work went
past it.

This is a different failure from the stale notes found before it. Those were
wrong when read against the corpus — the HL-C376 headword method catches them,
because the claim ("the corpus does not teach X") is false and a grep proves it.
These two were **true when written**. No search of the corpus for the point's own
exponents would have flagged them, because the exponent genuinely was missing at
the time. What changed was somewhere else.

Two things to do differently.

**After closing a point, grep the inventory for its id.** Any note that mentions
the point you just closed is now suspect, and the check costs one command:

```bash
grep -n "HI-A1-POST-04" code/learning/human-languages/core/exam-inventory-*.json
```

`ADJ-09` would have surfaced immediately, the same day `POST-04` landed.

**Treat "X is untaught" in a note as a dependency, not a verdict.** A note of
that shape is a dated claim about another part of the corpus, and it decays
every time that part changes. The notes that survive are the ones stating
something about the *point* — "the source's exponents are A, B and C" — rather
than something about the corpus's current state. When writing a note, prefer the
first shape; when reading one, check the date against what has landed since.
