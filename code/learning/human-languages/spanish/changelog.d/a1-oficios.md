## Unreleased — chapter 427: people named by what they do, and three points that needed no lessons

Spanish A1 exam coverage **255/273 (93%) → 262/273 (96%)**. Seven points close,
and **three of the seven cost no authoring at all**.

### Three notes were false, and the method that found them is the point

`A1-NE13-03`, `A1-NE15-04` and `A1-NE07-06` each enumerate **one** exponent, and
each of those three was already a lesson headword:

| point | exponent | taught by | what its note said |
|---|---|---|---|
| A1-NE13-03 | farmacia | `ES-C353-farmacia` | *"which the corpus never introduces"* |
| A1-NE15-04 | pescado | `ES-C361-pescado` | *"The corpus never introduces it"* |
| A1-NE07-06 | ser trabajador | `ES-C423-trabajador` | *"the adjective is never derived"* |

All three notes were wrong. Two had been wrong for many chapters; the third went
stale the moment chapter 423 landed *trabajador*, and nothing re-read the note.

This is the **HL-C375** class — coverage already earned and merely unwired —
found by the **HL-C376** method: resolve every exponent through a lesson
`headword:`, never through an atom id. The method has now paid for itself twice,
and it is worth restating why it is not optional: a null whose stated reason has
expired is **worse** than a bare null, because the note is exactly what a reader
trusts to prove the null was considered.

### The chapter is organised by how the names were built

Four points were genuine authoring: `A1-NE15-02` (commerce), `A1-NE11-04`
(protection and security), `A1-NE17-02` (law) and `A1-NE15-03` (businesses).

Six words, and they are grouped by **construction** rather than by occupation,
because that is the only thing that makes an unfamiliar profession word
readable:

| word | built from |
|---|---|
| el vendedor | the verb *vender*, plus **-dor** |
| el comprador | the verb *comprar*, plus **-dor** |
| el bombero | the noun *bomba*, plus **-ero** |
| el policía | the institution, *la policía* |
| el abogado | nothing in Spanish — inherited from Latin whole |

Spanish has no single way of naming a profession. What it has is a small set of
methods, and knowing them lets a reader **take any profession apart** even when
they could not have predicted it. Only **-dor** is productive for the learner:
give it a verb and it returns the person.

That half is a direct continuation of chapter 423, where **-dor** was met as an
adjective on *trabajador*. Here it does the job it was built for.

### Two facts a paper can actually test

**The article carries the whole distinction on *policía*:**

| | meaning |
|---|---|
| **el** policía | a police officer |
| **la** policía | the police **force**, or a woman officer |

The word never changes. Nothing else in the chapter behaves this way — the
**-dor** and **-ero** words move their endings and let the article follow.

**A bare profession takes no article:**

> *Soy abogado.* — not *Soy **un** abogado.*

English says *I am a lawyer*. Spanish drops the article when the sentence states
nothing but the profession, because the bare noun is a description rather than
one individual. The article returns the moment an adjective is added: *Soy un
abogado trabajador* — which also retrieves *trabajador* in its adjective sense.

### Two smaller notes worth keeping

**Bombero** is named after **la bomba**, the *pump*, not the fire — Spanish names
the person by what they carry where English names them by what they fight.
**Abogado** is Latin *advocatus*, *one called to your side*, the friend you
summoned to speak for you in a Roman court; English kept the same word whole as
*advocate*.

### Pins

| pin | before → after |
|---|---|
| `coverage.covered` | 255 → 262 |
| `coverage.percent` | 93 → 96 |
| `coverage.unmapped` | 18 → 11 |
| mock-audit `lessonCount` | 994 → 1002 |
| mock-audit `taughtForms` | 1669 → 1687 |

### Terminal-lesson debt, and the one count that moves

A *repaso* and a *síntesis* follow the six word lessons. Measured against a
clean-main equivalent: `attained` held at **A1**, A2 reinforcement shortfall
**flat at 33**.

`measurement-blind` **13 → 14**, the one new *síntesis* lesson. HL-C377 again;
not relabelled. **`forward-language` did not move at all** — the headword check
from the last tranche was run before writing, and none of the six new headwords
is a common word the corpus already uses.

### Verification

`human-language-data` 145 files / 2083 tests; `language-ladder` `bash BUILD`
39 files / 442 tests; all twelve `check:*` gates; `check-book-compile.sh
spanish` under XeLaTeX. The pre-generation checks caught two uses of a banned
word and three unregistered `sounds` tags, all fixed before generating.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
