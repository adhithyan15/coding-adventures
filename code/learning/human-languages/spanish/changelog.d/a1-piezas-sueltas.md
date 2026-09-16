## Unreleased — chapter 428: five last pieces

Spanish A1 exam coverage **262/273 (96%) → 266/273 (97%)**. Four points close,
and every one of the five words is the **last missing piece** of something the
corpus could otherwise nearly do.

| point | what it needed |
|---|---|
| `A1-NE09-06` | *arroba* and *guion bajo* — two of seven exponents |
| `A1-F5-10` | *felicidades* |
| `A1-F5-09` | *salud*, as a toast |
| `A1-NE20-05` | *animal*, the superordinate |

### Another stale note, found the same way

`A1-NE09-06` lists seven exponents. Five were already taught — *internet* and
*correo electrónico* since chapter 393, *punto*, *guion* and *página web* across
chapters 342, 422 and 424. Its note still claimed *guion* and *página web* were
missing.

That is the **second** stale note found in two tranches by the HL-C376 headword
method. It is becoming clear that the notes are the least-maintained part of the
inventory, precisely because nothing re-reads them when a chapter lands.

### Two points closed on words the corpus already had

This is the shape worth naming. **`feliz` was taught. `la salud` was taught.**
Neither covered its point, and both notes said why:

> *"The corpus teaches feliz as an adjective and never the congratulation
> formula."*
> *"…the noun does not demonstrate the act."*

An adjective does not congratulate, and a noun does not toast. The fix in each
case is the same word **put to a different use**:

| word | as taught | as an act |
|---|---|---|
| felicidad | happiness | **¡Felicidades!** — pluralised into a wish |
| la salud | health | **¡Salud!** — the article dropped |

**Coverage is about what the corpus can *do*, not only which strings it
contains.** The *salud* lesson declares a `introduces_senses` entry as well as
its atom, because that is what it is: a new sense, not a new word.

*¡Salud!* also carries the use an English speaker does not expect — Spanish
answers a **sneeze** with the same word, since both occasions are wishing
somebody health.

### The address is finally readable aloud

| symbol | said aloud | from |
|---|---|---|
| @ | arroba | this chapter |
| _ | guion bajo | this chapter |
| . | punto | chapter 424 |
| - | guion | chapter 424 |

**La arroba** is the pleasure of the chapter: it was a unit of weight, about
eleven and a half kilos, from Arabic *ar-rubʿ*, "the quarter." Merchants wrote
it with a looped **a**, and that shorthand is the symbol on the keyboard. A
Spanish speaker reading out an e-mail address is naming a **medieval unit of
weight**.

**El guion bajo** is named by position where English names it by action: the
*low hyphen*, against *under-score*.

### One count rose, knowingly and for a known-false reason

`forward-language` **376 → 389**, and all thirteen are `animal`, all in English
teaching prose: *"an unfamiliar animal"*, *"the colour of the animal"*, *"the
animal before this one"*.

This is **HL-C378** — an English word spelled like a target-language headword —
and it is its largest instance so far. Unlike the previous cases, **it was
predicted before the lesson was written**: the standalone-occurrence count was
run first and returned twenty-nine files.

The word was authored anyway, and that is a deliberate call worth recording.
Declining to teach an exam exponent because a detector known to be wrong will
produce noise would let the measurement dictate the curriculum — the exact
inversion this campaign has spent nine tranches avoiding. `forward-language` is
report-only, the cause is named here, and the fix HL-C378 proposes would clear
all thirteen at once.

The other four new headwords produced **zero** forward references, which is what
the pre-write check is for.

### Pins

| pin | before → after |
|---|---|
| `coverage.covered` | 262 → 266 |
| `coverage.percent` | 96 → 97 |
| `coverage.unmapped` | 11 → 7 |

`measurement-blind` **14 → 15**, the one new *síntesis* lesson. HL-C377 again.
`atomsNeverRevisited` improved, 92 → 91.

### What is left, and what is meant to stay

Seven points. Five are ordinary work: the affirmative imperative (`A1-F4-01`),
educational institutions, unemployment, clothing, and examination marks.

**Two are meant to stay null.** `A1-F2-10` and `A1-F6-06` argue in their own
notes that A1 has no linguistic exponent to probe — the first because plain
assertion is not an isolable act, the second because the source lists only
non-verbal behaviour. Wiring either would be the over-claim refused at
`A1-NE18-06` and again at `A1-NE06-05`.

### Verification

`human-language-data` 145 files / 2083 tests; `language-ladder` `bash BUILD`
39 files / 442 tests; all twelve `check:*` gates; `check-book-compile.sh
spanish` under XeLaTeX. Pre-generation checks caught one banned word; the
sounds-tag check ran and found none unregistered.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
