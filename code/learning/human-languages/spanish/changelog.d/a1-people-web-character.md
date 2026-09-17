## Unreleased — the two people chapter 420 refused to claim, a web page, and six adjectives

Spanish A1 exam coverage **234/273 (86%) → 237/273 (87%)**. Chapters 421–423
close three points, and each closes for a different reason.

| point | label | what it needed |
|---|---|---|
| `A1-NE18-06` | cinema and theatre | *actor*, *actriz* |
| `A1-NE16-02` | computing and new technology | *página web* |
| `A1-NE02-01` | character and personality | six of eight adjectives |

### The point that was left open on purpose

Chapter 420's changelog said this out loud: *"A1-NE18-06 is deliberately left
uncovered — it also wants actor and actriz, and neither is a headword
anywhere."* The corpus had *cine*, *teatro* and *película*, and wiring the probe
on those three would have repeated the over-claim a review blocked on
`A1-NE06-05`. Chapter 421 authors the two missing words, so the point closes by
teaching rather than by relaxing the claim.

**el actor** is Latin *actor* from *agere*, "to do" — not a theatre word at all
but the ordinary Latin agent noun, the one that also gives *agent*, *agenda*
and *action*. The stage borrowed it.

**la actriz** is the interesting half. The ordinary Spanish habit would give
*actora*; Spanish instead inherited Latin **-trix** whole and turned the **x**
into **z**. The ending is nearly extinct — *actriz*, *emperatriz*, *institutriz*
— and English kept it in *aviatrix* and *matrix*.

### The point that was one word short

`A1-NE16-02` lists four exponents and the corpus had taught three of them since
chapter 393: *ordenador*, *internet*, *correo electrónico*. It had stood
uncovered on **one missing word** for as long as the inventory has existed.

*Página* is Latin *pagina*, from *pangere*, "to fasten" — a column of text was
writing fixed into a frame, the way a vine-grower fastens shoots to a trellis.
The same verb gives *pacto* and *paz*. *Web* is English taken whole, and the
chapter makes the practical point: *web* sits in an adjective's chair without
being one, so it never agrees — *dos páginas web*, with no **-s**.

### The point whose note was wrong

`A1-NE02-01`'s note read *"The corpus introduces none of them."* Two of the
eight were headwords: **simpático** (`ES-C380-simpatico`) and **alegre**
(`ES-C380-alegre`). The note is corrected here — but the point closes on the six
lessons chapter 423 authors, not on the correction. This is the HL-C376 class of
error, and the audit that caught it resolved each exponent through a lesson
`headword:` rather than through an atom id.

The six are *antipático*, *inteligente*, *trabajador*, *serio*, *tímido* and
*sociable*, and the chapter is built so the eight demonstrate all three
agreement shapes at once:

| ending | example | feminine |
|---|---|---|
| **-o** | tímido | tímida |
| **-e** / **-able** | inteligente, sociable | unchanged |
| **-dor** | trabajador | trabajadora |

*Trabajador* earns its place twice: **-dor** is the same Latin **-tor** that
built *actor*, worn down by centuries of Spanish mouths, so chapter 421 and
chapter 423 turn out to be the same ending at two different ages. *Actor* came
in through books and kept the old spelling; *trabajador* grew at home.

### Terminal-lesson debt, budgeted rather than discovered

Chapter 420 learned this the expensive way — three new atoms at the end of the
track dropped Spanish's attained level from A1 to pre-A1. Here the retrieval
lessons were planned in from the start: each chapter carries a *repaso* and a
*síntesis* after its word lessons, and every new atom gets at least two later
retrievals.

Measured against a clean-main equivalent rather than assumed:

| | before | after |
|---|---|---|
| attained level | A1 | **A1** |
| reinforcement shortfall (A2) | 33 | **33** |
| headwords at or below A2 | 780 | 789 |

The shortfall stayed **flat**, which is the evidence the budget was right.

### Pins moved

| pin | before → after |
|---|---|
| `coverage.covered` | 234 → 237 |
| `coverage.percent` | 86 → 87 |
| `coverage.unmapped` | 39 → 36 |
| mock-audit `lessonCount` | 954 → 969 |
| mock-audit `taughtForms` | 1580 → 1610 |

### A gate caught something worth fixing rather than pinning

The narration refusal count went **44 → 49**: five of the first-draft tables
were four columns wide, which is the shape `chapter-policy` calls unspeakable,
so the narrator refused to read them aloud. One of them also carried its
content under an unlabelled first column — the exact shape the pin's own
comment names.

All five were reshaped to three labelled columns, and the count is back to
**44**. The pin was not touched. A reshaped table loses nothing: the *repaso*'s
character list drops a yes/no column and asks the reader to compare the feminine
against the masculine instead, which is the same observation done by looking.

### Two counts went up, and both stay up

`measurement-blind` moved **7 → 10** — the three new *síntesis* lessons. This is
HL-C377 exactly: `isExplicitRetrievalOnlyLesson` in `ramp.ts` accepts `review`,
`practice` and `practice-mix` but not `synthesis`, so a lesson that introduces
nothing still counts blind. As in chapter 420, the type was **not** changed to
`review` to clear the count.

`forward-language` moved **367 → 371**, and all four new entries are false
positives caused by this chapter: the English word *actor* appears in the
teaching prose of four long-merged lessons — *"an actor's mask"* is the gloss of
*persōna* itself — and the detector cannot tell that the token is English.
Recorded as **HL-C378**. The four lessons were not reworded; degrading real
teaching prose to satisfy a detector is fixing the measurement rather than the
thing measured.

### Verification

`human-language-data` 145 files / 2083 tests; `language-ladder` `bash BUILD`
39 files / 442 tests; `check:books`, `check:narration`, `check:modality`,
`check:shards`, `check:doc-shards`, `check:gentle-snapshots`,
`check:assessment-artifacts`, `check:progress`, `check:figures`,
`check:script-owner-evidence`, both mock audits, and `check-book-compile.sh
spanish` under XeLaTeX — all green. Spanish script-closure violations: 0.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
