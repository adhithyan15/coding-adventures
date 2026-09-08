## HL-C358 — The ordinal column after four more repairs: seventeen covered, and the order a language dictates is never the same reason twice

**MEASURED WITH HL-C350'S OWN CHECKED-IN SCRIPT, RE-RUN AGAINST `origin/main`
AFTER the Kannada and Malayalam tranches merged.** Re-run it rather than trusting
these figures:
`node code/learning/human-languages/data/scripts/numeral-probe.mjs`.

HL-C350 found ordinals the weakest single column in the corpus: twenty tracks
enumerate an ordinal point and eighteen left it uncovered. **The column now
stands at 17 points covered and 9 open**, and two of the nine close when the
tranches already in flight land.

**FIRST, A CORRECTION TO HL-C354, AND THE REASON IT MATTERS.** That entry listed
`urdu` and `russian` as waiting on CARDINALS. They are not any more: both gained
their cardinals AND their ordinals while HL-C354 was being read, and the probe
now scores `UR-A1-Q-03` and `RU-A1-Q-02`/`RU-A1-L-09` as covered. A reader who
had trusted the table would have gone looking for a cardinal tranche that already
existed. **This is the third time in this campaign that a filed table aged faster
than the work it described**, which is exactly why HL-C350 shipped a script
rather than a number, and the lesson to carry forward is narrower than "re-run
the probe": *the per-track rows of a status entry age at the speed of the
repository, and only the METHOD in it keeps.*

**THE COLUMN, MEASURED.** Covered (17): arabic, hindi, italian ×2, kannada,
latin ×2, malayalam, persian, portuguese, russian ×2, spanish ×3, telugu, urdu.
Open (9): `bengali BN-A1-Q-04`, `gujarati GU-A1-NUM-05`, `japanese JA-A1-NUM-03`,
`marathi MR-A1-QU-04`, `marwadi MW-A1-Q-02`, `punjabi PA-A1-NUM-05`,
`sanskrit SA-A1-Q-02`, `tamil TA-A1-NUM-04`, and `italian IT-A1-ORT-16`, which is
a TYPOGRAPHY point (the superscript ordinal indicator) rather than a vocabulary
one and wants a different repair. Tamil closes with the tranche in flight.

**THE FINDING THE FOUR NEW REPAIRS ADD TO HL-C354'S.** That entry said the order
is never numerical and each language's own construction dictates it. That is
true, and the four tracks repaired since sharpen it: **the order is dictated by
what the READER already has, not only by what the LANGUAGE has.** Two tracks with
almost identical grammars ordered their chapters in opposite directions:

- **kannada** — the ending `-ಅನೆಯ` has no exceptions; the one irregularity is
  that *first* is built on **ಮೊದಲು**, "a beginning", not on **ಒಂದು**. The reader
  had never met that word, so the chapter opens on **second**: the rule has to be
  visible before an exception to it can be. *first* lands third.
- **tamil** — the same shape, and the opposite order. Tamil's *first* is
  **முதல்**, and the reader had been saying it for twenty chapters inside
  **முதலில்**, "first of all", which `TA-A1-NUM-04`'s own note recorded as a
  DISCOURSE word. So Tamil opens on **first**, and the lesson teaches no new
  word at all: it takes **முதலில்** apart. The exception is the one item the
  reader already owned.
- **malayalam** — `-ആം` has no exception *anywhere*, not even at one, so
  Malayalam is the single track in the column whose set can honestly be taught in
  numerical order. Its first lesson opens on *first* in order to say so.

**A NOTE THAT NAMES A GAP MAY BE NAMING A HEAD START, AND IT IS WORTH RE-READING
EVERY UNCOVERED NOTE IN THAT LIGHT.** Tamil's note existed to say the ordinal was
missing. Read again it says exactly where the missing word already was. Portuguese
had the same shape (five of ten already in the mouth as weekdays) and so, it turns
out, does at least one still-open track: `punjabi` and `gujarati` should be read
for a *pehlā*-shaped word before either is priced.

**THE FOUR-WAY DRAVIDIAN BOX IS NOW PRINTABLE, AND IT IS THE PAYOFF OF DOING THE
FAMILY TOGETHER.** With telugu, kannada, malayalam and tamil all repaired, one
table says something no single track could: Tamil kept **முதல்** meaning *first*;
Kannada and Telugu built their ordinals on it (**ಮೊದಲನೆಯ**, **మొదటి**); Malayalam
kept the cognate **മുതൽ** and moved its sense to "**from**", filling the slot with
its ordinary ending instead. **None of the four builds "first" on its own word
for ONE.** That box is in the Tamil and Malayalam books, and it exists only
because the tranches were done as a set rather than one at a time.

**TWO METHOD FINDINGS THAT ARE NOT ABOUT ORDINALS.**

- **A CHARACTER CENSUS AND A GLYPH-CLOSURE GATE ANSWER DIFFERENT QUESTIONS, AND
  THE MALAYALAM TRANCHE FOUND OUT THE HARD WAY.** HL-C353 established that a
  coverage column tells you what is TAUGHT, not what is REQUIRED, and that the
  fix is to census the corpus's own characters including worked examples. That
  census passed for Malayalam — every character of every word in the tranche was
  already on the page. The corpus-wide glyph-gap queue still went non-empty,
  because **ഏ** is a *recognition-only* row in `data/scripts/malayalam.json`: it
  has no sourced stroke order, so a HEADWORD containing it cannot enter closure.
  A census answers "has the reader seen this shape?"; the closure gate answers
  "has the reader been taught to WRITE it?". **Both are needed, and a track can
  pass one and fail the other on the same letter.** A Donald R. Davis Jr.
  handwriting clip for ഏ exists on the source the other Malayalam vowels cite,
  but it is a video and could not be observed, so no stroke order was invented:
  the lesson is headed by its romanization, following the precedent that track's
  own counting lessons already set, and says on the page that the letter is to be
  read and not yet copied.
- **A GENERATED LEDGER IS NOT A LESSON.** The Marwadi tranche seated its
  reinforcement payments in the frontmatter, and the measurement went green while
  FIFTEEN lessons assessed atoms the learner was never told to retrieve. The
  reinforcement metric reads `practises.knowledge` and the block `assesses`
  lists; neither can tell whether a `[YOU RECALL: …]` line exists in the prose
  beside them. **A per-lesson audit that compares each block's assessed atoms
  against the prose around it is the check that catches this**, and it should run
  on any tranche that seats retrieval by table rather than by hand.

**WHAT THE FOUR REPAIRS COST, IN THE ONE NUMBER THAT MATTERS.** Every one landed
with **zero created reinforcement debt and zero newly-exposed debt**, verified by
walking every atom rather than by watching totals. Marwadi held an all-zero
strict report at 341 lessons — which took a 29th lesson (an atom introduced by a
track's final lesson can never be revisited) and a forward-reference fix (a
*hear* lesson printed the new word in Devanagari one lesson before the writing
lesson taught it). Tamil's four windows held to the unit. Kannada paid down two
R4 defects and Malayalam five, and in both cases **the atoms paid down were the
CARDINALS and the DIGITS** — taught once and never needed again until an ordinal
gave them something to do.

**WHAT THE QUEUE STILL HOLDS, and what each of the seven vocabulary points is
waiting on.** None is blocked on cardinals any more. `bengali`, `gujarati` and
`punjabi` reach five; `marathi` and `sanskrit` reach twenty and ten; `japanese`
teaches no numeral at all and its point wants the ordering words as much as the
ordinals; and `marwadi` is the one genuine SOURCING block in the column — its
signs are all taught and no citable Marwari-specific ordinal series has been
found in two separate searches. **Do not invent one.**
