## Unreleased — two A1 exam points closed, and they cost two different things

Chapter 419 moves Spanish A1 exam coverage from **229/273 (84%)** to **231/273
(85%)**, against the Instituto Cervantes A1 inventory. The two points are worth
separating, because they did not cost the same.

### A1-NE18-02 — music and dance, which needed a lesson

`ES-LEX-MUSICA` was already taught by chapter 413. `ES-LEX-BAILAR` did not exist
anywhere in the corpus, so the point genuinely needed authoring.

`ES-C419-bailar` teaches it as a **contrast rather than a word in a list**. The
learner already knows *me gusta la música* keeps an article English throws away;
this lesson is where that stops being true:

> *Me gusta **la** música.* — article.
> *Me gusta bailar.* — no article.

Not an exception to memorise: *gustar* needs a subject, a noun needs its article
to be spoken of in general, and an infinitive already names the action. The
etymology carries the same weight — Late Latin *ballare* gives English *ball*
(the dance), *ballet* and *ballad* (a song you danced to), while the *ball* you
throw is unrelated Germanic.

### A1-NE18-05 — photography, which needed nothing

`ES-LEX-FOTO` and `ES-LEX-FOTOGRAFIA` have been introduced by chapter 405 all
along. The point's `probe` was `null`, so the inventory reported the track could
not do something it had been teaching for over a hundred chapters. Wiring the
probe is the entire change.

That second case is not a one-off. A corpus-wide scan puts **451 of the 2,189
uncovered points (21%)** in the same shape — at least one already-introduced
atom whose name matches the point's label. Recorded as HL-C375 in the backlog,
where the next tranche is a probe-wiring audit rather than authoring.

### Pins moved

| pin | before → after |
|---|---|
| `coverage.covered` | 229 → 231 |
| `coverage.percent` | 84 → 85 |
| `coverage.unmapped` | 44 → 42 |

Each is recorded beside the reason in `tests/exam-inventory.test.ts`. No
assertion was loosened, and `latex-warning-baseline.json` and the banned-word
and info-dump ceilings are untouched: the draft of this lesson tripped both the
banned-word ceiling (`just`) and the info-dump rule-statement ceiling (`the rule
is`), and both were fixed in the prose rather than absorbed into the number.
