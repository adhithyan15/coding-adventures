## Unreleased — chapter 94: the pronouns, and the article Hindi does not have

Hindi A1 exam coverage **206/282 (73%) → 209/282 (74%)**, closing three points.

| point | what was missing |
|---|---|
| `HI-A1-PRON-04` | the subject paradigm — *ham* and *ve* |
| `HI-A1-PRON-06` | the oblique pronouns |
| `HI-A1-DET-01` | Hindi has no article |

### Two of the three were findings rather than gaps

`HI-A1-PRON-06`'s note said *mujhe* existed only inside the *pasand* frame
rather than as a paradigm. Following that up turned out to be the chapter's
best moment: **the frame was never a lump.** *Mujhe* is **मैं** in its oblique
form, meaning *to me*, and once the rest of the set is visible the liking
question can be asked of anybody:

| subject | oblique |
|---|---|
| मैं | *mujhe* |
| हम | हमें |
| तुम | तुम्हें |
| आप | आपको |
| वह | *use* |

Two show **को** plainly and three take the nasal ending. Only *mujhe* and *use*
look irregular, and they look that way because they wore down — *maiṅ + ko* and
*us + ko* squeezed together by centuries of use.

`HI-A1-DET-01` is described in its own note as *"the single largest structural
difference from the proxy language,"* and the corpus had never stated it.
**Hindi has no article.** A bare *kitāb* covers both *a book* and *the book*.
Three things carry specificity when it matters — **एक**, **वह**, or nothing at
all — and the lesson makes the point the note asked for: *ek chāy dījiye* is
*ek* the **numeral** doing indefiniteness work, not a café formula.

The lesson ends on the habit rather than the fact: **the commonest correct
translation of English *the* is nothing at all.**

### The fourth appearance of one decision

| system | at the tum level | at the āp level |
|---|---|---|
| possessive | तुम्हारा | आपका |
| copula | हो | हैं |
| command | बोलो | बोलिए |
| **oblique pronoun** | **तुम्हें** | **आपको** |

Four systems, one decision. The review lesson sets them side by side rather than
teaching a fourth list, which is what three chapters of grid-building were for.

### The item that was unreadable from its first word

`HI-A1-PRON-04`'s note names it: *mock 2 reading item 10 is `ham station par
milenge`*. The synthesis lesson reads that sentence, and points at what is **not**
in it — no word for *the* before *sṭeśan*, because there is none to put there.

The *ham* lesson also carries a use an English speaker will not expect: *ham*
often stands in for *maiṅ*, said modestly or as part of a household. It is the
same instinct that made *hamārā parivār* the natural way to say *my family*.

### One forward reference, and it is ours

`forward-language` **11 → 12**. The entry is genuine and self-inflicted:
**`HI-C84-hamara` prints हम** to show how *hamārā* is built, thirteen lessons
before this chapter introduces it.

The honest reading is that the ordering should have been the other way round —
the pronoun before the possessive built on it. Chapter 92 is merged, so the cost
is paid rather than avoided, and the new lesson at least turns it into a
callback: *"you have been carrying it for a chapter without being given it."*

**Worth carrying forward:** when a lesson explains a word by naming its parts,
those parts are forward references unless they are already taught. Check the
components, not only the headword.

### Terminal-lesson debt, budgeted

Five lessons introduce one atom each; two retrieve. Measured against a
clean-main equivalent: `attained` unchanged, reinforcement shortfall **flat at
44**. Pre-A1 headwords rose **191 → 196**, which also moves the pre-A1
vocabulary shortfall 109 → 104.

`measurement-blind` **7 → 8**, the one new *síntesis* lesson. HL-C377 again; not
relabelled.

### Gates caught three things before generation

A **spine-node mismatch** — the article lesson declared `SPINE-DEFINITE-REFERENCE`
while its path declares `SPINE-EXCHANGE-NAMES`, and every lesson in a path must
declare the path's node. A **prerequisite chain gap** — *tumhārā* is
*downstream* of *hamārā*, so reaching it required naming it directly rather than
relying on the chain. And one **banned word** in a table cell.

Script closure: **zero new debt**. The two untaught glyphs in this chapter's
words — **झ** in *mujhe* and **उ** in *use* — appear only in headwords, where a
declared romanization makes them exposure; body prose romanizes both.

### Pins

| pin | before → after |
|---|---|
| Hindi lesson count | 394 → 401 |
| `coverage.covered` | 206 → 209 |
| `coverage.unmapped` | 76 → 73 |
| report string | 206/282 → 209/282 |

### Verification

`human-language-data` 145 files / 2083 tests; `language-ladder` `bash BUILD`
39 files / 442 tests; all twelve `check:*` gates; `check-book-compile.sh hindi`
under XeLaTeX.

### What this does not claim

Coverage means the teaching exists, not that a reader scores. No track has had a
single exam item graded against it.
