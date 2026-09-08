## HL-C353 — Marwadi's market finally has numbers in it: twelve cardinals and ten digits for ONE new letter

**HL-C350 named marwadi the most urgent of the seven zero-numeral tracks and
the most absurd of them. This closes it, and the interesting part is the
price.** Chapters 26--31 are a market -- *how much is this?*, *make it a little
cheaper*, *what final price will you give?*, *take the money* -- and **एक, दो,
तीन, चार, पांच** returned ZERO occurrences across 257 lesson files. The
seventeen raw matches for *do* were the imperative verb *do*, "give".

Chapters 32--36 add 55 lessons and teach **twelve cardinals** -- *ek, do, tīn,
chār, pāṅch, chha, sāt, āṭh, no, das, bīs, so* -- the **ten Devanagari
digits**, a one-word price answer, and a counter-offer.

**IT COST THE HAND ONE LETTER.** Independent **ए**, for **एक**. Every other
sign in all twelve number words was already in the track: **दो** is the asking
word's own spelling, **आठ** is chapter 2's **आ** beside chapter 17's **ठ**,
**दस** is two's **द** beside seven's **स**, and **सो** is two signs. A census
of the track's 41 distinct Devanagari characters BEFORE designing anything is
what found that, and it is the same finding Gujarati's coverage column produced
-- **a coverage column tells you what is TAUGHT, not what is REQUIRED.** Read
the worked examples, not the headwords. Had the ratio been read the other way
round, this tranche would have been scheduled behind five script lessons it did
not need.

**TWENTY BEFORE THE TEENS, BECAUSE THE LANGUAGE SAYS SO.** Marwari counts DOWN
to a round number: nineteen is *ughaṇīs*, one short of twenty, and Turner
records *ekuṇcālīs*, one short of forty, for thirty-nine. So **बीस** has to
exist before the numbers under it, and teaching eleven to nineteen in numerical
order would have printed it in bold in a lesson that had not taught it. The
pedagogy and the metric wanted the same thing here, as they did for Marathi's
*ekoṇīs*.

**WHAT IS LEFT UNCOVERED, AND WHICH KIND OF GAP EACH ONE IS.** These are two
different problems and a note that said only "absent" would send the next
tranche after the wrong one.

- **Eleven to nineteen: a SCRIPT gap, priced.** *igyārā* needs **इ**, *aṭhārā*
  needs **अ**, *ughaṇīs* needs **उ**, *soḷā* needs **ळ**. Four independent
  vowels and one consonant, so five script lessons — and **बीस** is already
  taught, so nothing has to be reordered when they land.
- **Ordinals: a SOURCING gap, and it costs nothing to fix.** *pahlo, dūjo,
  tījo, chautho* need NO sign this track has not taught — प ह ल ो, द ू ज ो, त
  ी ज ो, च ौ थ ो are all in the corpus. No citable Marwari-specific ordinal
  series was found; the Rajasthani numeral list this tranche cites prints
  cardinals only. Ordinals are the weakest column in the whole corpus (twenty
  tracks enumerate one, eighteen leave it uncovered), and for this track the
  block is a source rather than a sign.
- **Still no word for NO.** Untouched, and it matters MORE now: a course that
  can name a price and cannot decline one has half a bargain.

**THE REINFORCEMENT DECOMPOSITION, because a rise has two causes and only one
of them is yours.** Marwadi is one of two tracks with an all-zero strict report,
and it still is at 312 lessons. Growing 257 → 312 made **42 R4 windows and 11 R3
windows judgeable for the first time** — windows the shorter track was never
long enough to be measured on. Every one of them lands on a chapter 18--25 atom,
so the sweep was designed backwards from the constraint before a word was
written: each new chapter's warm-ups retrieve that older vocabulary in the order
it was taught, and the last two lessons of the tranche retrieve **लो**, the
bargaining request and **करो**, whose windows close last. The two defects that
did appear were: **one created by this tranche** (*das karo* missing its R2) and
**eight pre-existing** (chapter 24--25 atoms whose R3 windows had never been
judged), and the fix for all nine was the same — move the retrieval into the
window rather than in front of it.

**A FALSE POSITIVE WORTH RECORDING.** Teaching six as **छे** raised the track's
forward-reference count from 0 to 2, because **छे** stands emphasised on its own
in chapters 9 and 10 as the second syllable of **पाछे**. The source prints
**छ/छे**; teaching the bare **छ** as the headword removes the report (a
single-character headword is not a forward-reference matcher) and the lesson
names **छे** as the variant, so the reader gains the observation instead of the
tooling losing to it.

**THE MEASUREMENT MOVED IN THE SAME COMMIT AS THE AUTHORING, AND BOTH HALVES
WERE FALSIFIED.** `MW-A1-Q-01` and `MW-A1-LIP-13` move from `probe: null` to
probes naming real atoms; marwadi A1 coverage moves 74/197 (38%) → 76/197 (39%).
A fabricated atom id breaks the suite, and so does removing a probe — each was
tried. The corpus layer of `data/scripts/numeral-probe.mjs` now reports marwadi
at 12 numeral lessons and *highest* 100, against 0 and 0 before.

**WHAT THE WORK QUEUE STILL HOLDS.** HL-C350's remaining zero-numeral tracks:
`persian` and `urdu` teach no numeral at all, `russian` holds only *odin* as
half a joining pattern, and `chinese` teaches *yi*--*wu* and nothing from *liu*
to *shi*. `french` and `german` reach twenty against a point demanding a
hundred and want a different repair.
