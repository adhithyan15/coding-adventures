## HL-C354 — The ordinal column: five tracks repaired, thirteen still open, and what each one is waiting on

**MEASURED AGAINST `origin/main` AT `4a5d6814`, AND THE MEASUREMENT IS THE SAME
SCRIPT HL-C350 FILED.** Re-run it rather than trusting these figures:
`node code/learning/human-languages/data/scripts/numeral-probe.mjs` after building
the human-language-data package.

HL-C350 found ordinals the weakest single column in the corpus: **twenty tracks
enumerate an ordinal point and eighteen left it uncovered**, with only Arabic and
Spanish teaching one. Arabic's was covered sideways -- its weekday chapter teaches
"the first", "the second" as the days themselves, so the ordinals were never
detached from the week. That left ONE track, Spanish, teaching the series as a
series.

Five tracks have now had a tranche: **latin**, **portuguese**, **italian**,
**telugu** and **hindi**. Latin is merged; the other four were in flight when this
was filed, so re-measure before quoting a total.

**WHAT THE REPAIRS FOUND THAT A PER-TRACK READING WOULD NOT.** Every one of the
five had the ordinals sitting behind something the track had ALREADY taught and
never cashed in, and in four of the five the gloss was already on the page:

- **latin** -- chapter 11 glossed *Quīntīlis* as "the fifth" and *Sextīlis* as
  "the sixth" and never gave the words. *post* was quoted inside *post merīdiem*
  in chapter 35 and never taught.
- **portuguese** -- FIVE of the ten ordinals were already in the learner's mouth
  as weekdays, and chapter 7 says in as many words that *segunda*, *terça*,
  *quarta*, *quinta* and *sexta* are ordinals of numbers the learner has.
- **italian** -- the entire 93-lesson corpus contained ONE ordinal, `primo`,
  inside the gloss of *primavera*.
- **hindi** -- `pahle` "before" was reachable and `dūsrā` "other" was not, so the
  distributive *ek … dūsrā* needed no new word once the ordinal was taught.
- **telugu** -- one irregular word and one ending covers the whole number line,
  which makes it the cheapest ordinal column in the corpus and nobody had noticed.

**THE ORDER IS NEVER NUMERICAL, AND THAT IS THE FINDING.** Each track's own
construction dictates a different order. Telugu has ONE irregular and then a rule,
so the rule is lesson two. Hindi has FOUR irregulars, the rule arrives at five,
and it has exactly one hole at six (*chaṭhā*, inherited before the rule existed).
Italian has ten inherited words and then a completely regular `-esimo` above ten,
so the SEAM is the lesson. Portuguese's *terceiro* has a twin the week kept
(*terça*), so the exception is a lesson rather than a footnote. Teaching one to
ten in numerical order and stopping there hides the only structural fact in each
of those sets.

**THE THIRTEEN STILL OPEN, and what each is waiting on.** Three of them are not
ordinal work at all:

| track | point | what it is waiting on |
|---|---|---|
| marwadi | `MW-A1-Q-02` | CARDINALS. The track teaches no numeral at all, so an ordinal has nothing to build on. |
| persian | `FA-A1-Q-02` | CARDINALS, same reason. |
| urdu | `UR-A1-Q-03` | CARDINALS, same reason. |
| russian | `RU-A1-Q-02`, `RU-A1-L-09` | CARDINALS for the first; the second wants the WRITTEN ordinal, a numeral with its grammatical ending hyphenated on. |
| bengali | `BN-A1-Q-04` | nothing structural -- cardinals reach five and *prathama*/*dvitīya* are a small tranche. |
| gujarati | `GU-A1-NUM-05` | nothing structural; cardinals reach five. |
| japanese | `JA-A1-NUM-03` | the ordering words as much as the ordinals; `-me` and the counter it attaches to. |
| kannada | `KA-A1-NUM-07` | nothing structural. Cardinals reach twenty and the shape is Telugu's -- one irregular plus `-neya`. |
| malayalam | `ML-A1-NUM-05` | nothing structural; cardinals reach twenty. |
| marathi | `MR-A1-QU-04` | nothing structural, though cardinals reach only five and the point says "to tenth". |
| punjabi | `PA-A1-NUM-05` | nothing structural; cardinals reach five. |
| sanskrit | `SA-A1-Q-02` | nothing structural; cardinals reach ten and *prathama, dvitīya, tṛtīya* are the source the Indic tracks all descend from. |
| tamil | `TA-A1-NUM-04` | nothing structural; cardinals reach twenty. |

**THE CARDINAL TRACKS ARE UPSTREAM AND MUST GO FIRST.** Four of the thirteen
cannot be repaired by an ordinal tranche in any order, because an ordinal that is
"the cardinal plus an ending" has nothing to attach to. They are listed here so
the next reader does not spend the measurement twice.

**TWO THINGS THE REPAIRS LEARNED THAT ARE NOT ABOUT ORDINALS.**

- **Two of the five tracks had NO exam-coverage assertion at all.** Telugu and
  Hindi both had a `tests/corpus/<track>.test.ts` that pinned continuity and
  modality and never looked at the inventory, so wiring a probe would have moved
  a number nothing read. Both now pin the coverage total and check that every
  probe names an atom that exists, and both halves were falsified before being
  kept. **The other eighteen inventories should be checked for the same hole**;
  only Spanish is pinned corpus-wide, in `exam-inventory.test.ts`.
- **`core/main-font-charset.json` was missing two characters the font has.**
  `U+00AA` and `U+00BA`, the ordinal indicators, are in `lmroman10-regular.otf`'s
  cmap and were absent from the committed list, so `1.º` was unwritable in every
  book. They were added following that file's own `howToAdd` procedure, verified
  twice -- a fontTools cmap query and a XeLaTeX render with no missing-character
  warning. The file is a curated allow-list rather than the font's full cmap, so
  **it will have other holes of the same kind**, and each one silently forbids a
  character the books could set.

**A RENDERING DEFECT FOUND WHILE READING THE HINDI PAGES, and NOT fixed here.**
The romanization sequence `ā̃` -- U+0101 followed by U+0303 COMBINING TILDE -- sets
as `ã` in the compiled book: the macron is dropped and only the tilde survives.
That sequence is the Hindi track's established convention and appears in 25
committed lesson files (`vahā̃`, `yahā̃`, `kahā̃`, `sā̃p`, `kuā̃`, `chā̃d`, `gā̃v`),
so this is a pre-existing corpus-wide defect rather than one the ordinal tranche
introduced -- but it means a reader of the PDF cannot tell a long nasal vowel from
a short one anywhere in the track. The fix is a preamble mapping or a precomposed
character, and it is a Hindi-wide change rather than a tranche-sized one.
