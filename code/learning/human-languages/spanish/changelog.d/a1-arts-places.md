## Unreleased — three places close two A1 points, and a terminal-lesson debt gets paid

Spanish A1 exam coverage **232/273 (85%) → 234/273 (86%)**. Chapter 420 adds
*el teatro*, *la exposición* and *el circo*, closing `A1-NE18-01` (artistic
disciplines) and `A1-NE08-02` (shows and exhibitions).

Three words close two points because the corpus was already most of the way
there: six of NE18-01's eight exponents — *cine, música, película, concierto,
foto, museo* — were taught across chapters 353–413, and only *teatro* and
*exposición* were missing.

### What the names are actually about

| place | named after | the thing itself |
|---|---|---|
| el teatro | the audience | a looking-place |
| el cine | the machine | a movement-writer |
| el museo | the Muses | a seat of the arts |
| la exposición | the act | a putting-out |
| el circo | the floor | a ring |

Not one is named for the art that happens inside it. *Teatro* is Greek
*théatron*, from *theáomai* "to behold" — and the same root gives English
**theory**, so a theatre and a theory are the same idea twice: someone standing
back and watching. *Exposición* is *ex-* plus *ponere*, a putting-out, which
makes it transparent to a Spanish speaker in a way *exhibition* is not to an
English one. *Circo* is a ring, and happens to contain both a soft and a hard
**c**, so the chapter keeps it as the reference word for that rule.

### A1-NE18-06 is deliberately left uncovered

"Cinema and theatre" also wants *actor* and *actriz*, and neither is a headword
anywhere. Wiring it on *teatro* alone would repeat exactly the over-claim a
review blocked on `A1-NE06-05`. The note records what is now taught and what is
still missing.

### The terminal-lesson cost, measured

Adding three words at the end of the track dropped Spanish's **attained level
from A1 to pre-A1**. The level gate was precise about why: two atoms at or
below A1 were revisited fewer than twice, because an atom introduced last has
nowhere left to be retrieved.

Two retrieval lessons — a *repaso* and a *síntesis* — discharge it. They are not
padding: the repaso sorts the five places by what their names describe, and the
síntesis puts them in a real proposal where **al** and **a la** do the gender
work.

The same pass also pays an **older debt**. `ES-LEX-BAILAR`, introduced by
chapter 419, had **zero** revisits — taught once and never retrieved. Both new
lessons now retrieve it, and the síntesis earns it rather than bolting it on:

> —¿Y después del circo?
> —Después, a bailar.

*Bailar* is a verb, so it takes plain **a** and no article — the same reason
*me gusta bailar* needed none. The exception belongs in the lesson about
choosing between **al** and **a la**.

Attained level is back to **A1**, with blockers at A2 where they were before.

### Pins moved

| pin | before → after |
|---|---|
| `coverage.covered` | 232 → 234 |
| `coverage.percent` | 85 → 86 |
| `coverage.unmapped` | 41 → 39 |
| headwords at or below A2 | 777 → 780 |

Verified: `human-language-data` 145 files / 2083 tests; `language-ladder`
`bash BUILD` 39 files / 442 tests; all `check:*` gates and the glyph gate green.

One generator note: the first draft of the *teatro* lesson wrote **θέατρον** in
Greek script, and the glyph gate caught seven unrenderable characters — Spanish
has no Greek font. The corpus convention is to romanize, which every other
Greek etymology here already does.

### One count went up, and it stays up

`measurement-blind` moved **6 → 7**. The cause is `ES-C420-sintesis-salir`:
`isExplicitRetrievalOnlyLesson` in `ramp.ts` accepts `review`, `practice` and
`practice-mix` but not `synthesis`, so a lesson that introduces nothing still
counts blind. `ES-C57-sintesis-reportar` already sat in the same position.

The type was **not** changed to `review` to clear the count. The lesson is a
synthesis — it puts five separate words into one connected exchange and makes
the learner choose between *al* and *a la* — and relabelling it would be fixing
the measurement rather than the thing measured. Recorded as HL-C377, with the
`ramp.ts` change worth making when someone is next in that file.
