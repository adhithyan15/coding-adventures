## HL-C359 — Three ordinal tranches from one probe run, and the finding that the evidence for the order is in the dictionary

**MEASURED WITH HL-C350'S OWN CHECKED-IN SCRIPT, RE-RUN AGAINST `origin/main`
BEFORE ANY OF THIS WORK AND AGAIN AFTER IT.** Re-run it rather than trusting
these figures — HL-C358's whole point was that the per-track rows of a status
entry age at the speed of the repository:
`node code/learning/human-languages/data/scripts/numeral-probe.mjs`.

**WHERE THE COLUMN STOOD WHEN THIS WORK STARTED.** 17 covered, 9 open. By the
time the first tranche was authored Tamil had merged, so the probe read **18
covered and 8 open**: `bengali BN-A1-Q-04`, `gujarati GU-A1-NUM-05`,
`japanese JA-A1-NUM-03`, `marathi MR-A1-QU-04`, `marwadi MW-A1-Q-02`,
`punjabi PA-A1-NUM-05`, `sanskrit SA-A1-Q-02`, and `italian IT-A1-ORT-16`,
which is a TYPOGRAPHY point rather than a vocabulary one. **This entry repairs
three of the eight — marathi, sanskrit, punjabi — and prices the rest.**

**HL-C358 LEFT TWO SPECIFIC WARNINGS AND BOTH WERE WORTH ACTING ON, THOUGH ONE
CAME BACK NEGATIVE AND THAT IS ALSO A RESULT.** It said to read `punjabi` and
`gujarati` for a *pehlā*-shaped word before pricing either, because an "absent"
note can be describing a head start. **Both were read and both are genuinely
empty** — `ਪਹਿਲ`, `ਦੂਜ`, `ਤੀਜ`, `પહેલ`, `બીજ`, `ત્રીજ` and their romanizations
return zero hits across 261 and 263 lessons. But the same read found the head
start somewhere HL-C358 had not looked: **`marathi` already taught दुसरा in
chapter 31**, glossed there as *the other, **the second***, with an etymology
hook already saying it is दोन wearing an ordinal ending. The Marathi note said
"Untaught". *The lesson to carry is not "check the two tracks named"; it is that
the note which says a point is missing is written by somebody who was looking at
the point, not at the corpus — so the check has to be a corpus grep for the
WORD, run on every open track, not on the ones a previous entry guessed at.*

**THE FINDING THIS ROUND ADDS, AND IT IS A TECHNIQUE RATHER THAN A FACT.**
HL-C354 said the order is dictated by the language's own construction; HL-C358
sharpened it to what the READER already has. All three tranches here found their
decisive evidence in the same unexpected place: **a dictionary's ENTRY LIST is
direct, checkable evidence about which ordinals are words and which are
arithmetic**, because a dictionary records what a reader cannot derive.

- **marathi** — Molesworth's 1857 *Dictionary, Marathi and English* carries
  पहिला (p. 497), दुसरा (p. 420), तिसरा (p. 381) and चौथा (p. 296) as headwords
  and returns **no result** for सहावा, सातवा, आठवा, नववा or दहावा. The one place
  पाचवा appears at all it is a different word entirely — a returning sickness.
  The seam this tranche drew between चौथा and पाचवा was drawn in Marathi
  lexicography a century and a half earlier, and the chapter payoff prints that
  table so a reader can check it.
- **sanskrit** — Macdonell's *A Practical Sanskrit Dictionary* prints the JOIN
  inside every one of the ten headwords: *pañca-má*, *ṣaṣ-thá*, *dvi-tīya*,
  *katur-thá*, and for *first* a bracket the other nine do not get,
  *pra-thamá [= pra-tama, spv. foremost]*. The grouping into three endings plus
  one superlative is read off the dictionary rather than asserted.
- **punjabi** — Wiktionary derives all five whole from Sanskrit AND says of
  ਪੰਜਵਾਂ, in the same entry, that "by surface analysis" it is ਪੰਜ + -ਵਾਂ. Both
  halves are true and the second happened after the first.

**"INHERITED VERSUS BUILT" IS THREE DIFFERENT QUESTIONS, AND NAMING THE SEAM AS
"WHERE INHERITING STOPS" WOULD HAVE BEEN WRONG FOR TWO OF THE THREE.**

- **marathi** opens on SECOND and teaches no new word to do it, because दुसरा
  was already in the mouth; *first* arrives THIRD, because पहिला carries no एक
  and an exception is only legible after the shape has been seen holding twice.
  Four inherited words in three shapes, then -वा from five, with one adjustment
  at nine where नववा stands on the older नव rather than the modern नऊ.
- **sanskrit** opens at FIFTH and SKIPS SIX, saying so on the page, because -म
  is the ending that generalizes and a rule must be visible before a departure
  reads as one. Chapter 63 then works OUTWARD from the hole: षष्ठः is odd until
  चतुर्थः makes -थ a pair; तृतीयः is odd until द्वितीयः makes -तीय a pair; and
  प्रथमः lands last because it can only be seen as a different KIND of word once
  nine of the same kind have gone past.
- **punjabi** has NO built half at all. Every one of its five came down whole
  from Sanskrit, so the seam at five is where the inheriting stopped being
  VISIBLE: ਪੰਜਵਾਂ arrived in one piece, then looked like a sum, and -ਵਾਂ became
  productive on that reading. Its payoff therefore asks *which ending got a
  second life* rather than *which words were built* — and the seam is audible
  before it is explained, because the four inherited ordinals end in a plain -ਾ
  and ਪੰਜਵਾਂ ends in a nasal.

**WHAT THE THREE COST, IN THE NUMBER THAT MATTERS.** Every one landed with
**zero created reinforcement debt and zero newly-exposed debt**, verified by
walking every atom rather than by watching totals. Marathi held 342 with all
four windows unmoved across 21 newly-judgeable slots; Punjabi held 507 across 16
and additionally paid down **two never-revisited atoms**; Sanskrit went
**913 → 904** across 27, and **five of its six R4 payments were the cardinals
six to ten**. That is now the third campaign in a row where the R4 payments turn
out to be the numbers themselves: *a cardinal is taught once and never needed
again at distance until an ordinal gives it something to do.*

**A HAND-COPIED CENSUS IS NOT A CENSUS, and this is the one place the work
nearly went wrong.** The Punjabi tranche avoided the word ਚਾਹ because a
transcribed list of the track's taught glyphs had silently dropped ਹ. The corpus
teaches ਹ; the transcription was simply wrong, and the only reason it surfaced
was that `measureScriptClosure` disagreed with the hand list. **A census must be
derived from the corpus at the moment it is used, never copied from a previous
run's printed output** — the same class of error as the stale-table one HL-C358
recorded, one level down.

**AND THE GLYPH-COVERAGE GATE CATCHES WHAT SCRIPT CLOSURE CANNOT.** Three
Punjabi lessons quoted their Sanskrit ancestors in Devanagari. That passes
closure — it is a cousin script, not the track's own — and it fails the FONT: the
gate named fifteen characters the Punjabi book cannot render. The ancestors are
cited in IAST instead, which is also the better lesson, because a Punjabi learner
has not been taught Devanagari and the point of the citation is the shape of the
word. *Closure asks whether the reader was taught the letter; the coverage gate
asks whether the book can print it; a cousin-script quotation can pass the first
and fail the second.*

**ONE POINT PER TRANCHE, THREE TIMES, AND THAT IS NOT A SHORTFALL.** Twelve
lessons closed exactly one point in Marathi, twelve closed exactly one in
Sanskrit, six closed exactly one in Punjabi. **The ordinal column unlocks nothing
else**, and each inventory now says why in its own note rather than leaving the
ratio to be read as a miss: Marathi's ordering point wants ordering EXPONENTS
(आधी, नंतर, मग), not more ordinals; Punjabi's sixth-upward is blocked on
cardinals that stop at ਪੰਜ, and its coverage pin asserts `PA-A1-NUM-02` is STILL
uncovered so the ratio cannot be misread. **Sanskrit's note carried a stale claim
that had to be corrected**: it said the missing ordinals were also why
`SA-A1-SP-02` was uncovered. They were not. SP-02 is relative position — left,
right, above, below, in front, behind — and it is blocked on six direction words
nobody teaches.

**WHAT THE QUEUE STILL HOLDS, AND WHAT EACH ONE IS WAITING ON.** Five points,
and only two of them are ordinary vocabulary work:

- **`gujarati GU-A1-NUM-05` is READY AND CHEAP, and is the one to take next.**
  Cardinals reach five (chapter 12), all five ordinal headwords are clear against
  the track's own taught Gujarati glyphs, and English Wiktionary carries the
  etymologies. It is Punjabi's shape — four inherited plus -મું from five — so
  one chapter of six lessons prices it, and the Punjabi tranche is the template.
- **`bengali BN-A1-Q-04` is ready but NOT cheap, and the cost is script.** Its
  ordinals are Sanskrit tatsama with no cardinal visible in them at all —
  প্রথম, দ্বিতীয়, তৃতীয় — so none of the "can you see the number in it"
  machinery the other four tracks used is available, and the honest teaching
  order will have to be found somewhere else. পঞ্চম additionally needs ঞ, which
  the track shows but does not teach. **And HL-C212 stands: Commons has
  stroke-order animations for every Bengali VOWEL and none for any consonant**,
  so a writing lesson for ঞ must follow that track's own precedent — place of
  articulation plus a Unicode chart citation, claiming no pen path.
- **`japanese JA-A1-NUM-03` IS BLOCKED UPSTREAM, and this is the answer HL-C358
  asked for.** The track teaches ONE numeral in 117 lessons — the *ichi* inside
  *ichidō*, in a phrase — and `JA-A1-NUM-01`'s own note says so and marks itself
  thin for exactly that reason. `JA-A1-NUM-02`, the counters, is null: Japanese
  runs two parallel counting systems and choosing between them is the first hard
  thing about its numbers. **An ordinal needs a counter to attach to, so this
  point cannot be repaired before NUM-01 and NUM-02 are.** It was not attempted,
  and the right next move on this track is the cardinals and the two counter
  series, which is a much larger piece of work than an ordinal tranche.
- **`marwadi MW-A1-Q-02`** remains the column's one genuine SOURCING block, per
  HL-C358: the signs are taught and no citable Marwari-specific ordinal series
  has been found in two separate searches. **Do not invent one.**
- **`italian IT-A1-ORT-16`** is the superscript ordinal INDICATOR — a typography
  point whose vocabulary half is already closed by `IT-A1-Q-04`. It wants a
  different repair from everything above and should not be counted with them.
