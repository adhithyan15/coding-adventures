## HL-C360 — Japanese could count to five before this tranche existed, and two of the ten cardinals had been sitting in the reader's hands for nine chapters

**MEASURED WITH HL-C350'S OWN CHECKED-IN SCRIPT, RE-RUN AGAINST `origin/main`
BEFORE THIS WORK AND AGAIN AFTER IT.** Re-run it rather than trusting these
figures — HL-C358's whole point was that the per-track rows of a status entry age
at the speed of the repository:
`node code/learning/human-languages/data/scripts/numeral-probe.mjs`.

**THE ENTRY THIS ONE ANSWERS.** HL-C359 priced the remaining numeral work and
described Japanese as the one track that "teaches ONE numeral in 117 lessons"
with "no counter at all", ordinals blocked upstream behind a counter series. The
first half is exactly right and the probe still says so. **The second half was
wrong in the track's favour twice over, and both corrections were found by
reading the corpus rather than the note.**

**FINDING ONE: THE READER ALREADY OWNED TWO OF THE TEN, AND ONE OF THEM WAS
DEFERRED ON THE PAGE IN WRITING.** `JA-C09-ichido` teaches *ichido*, "one time",
and says in its own prose that *ichi* is **one**. And `JA-W05-five-component`
teaches the four strokes of **五** as the sound clue inside **語** — and its
Script block says, verbatim, *"This sign means five on its own. Here, you only
need its second job."* That is not a head start hiding in a gloss, as Marathi's
दुसरा was; **it is an explicit, written promise to come back**, made nine chapters
earlier, and nobody had come back. So the cardinal chapter opens on **ichi** and
**go**, spends no sign on either, and the second lesson's whole job is to hand
over the job the chapter-5 lesson said it was holding.

*The lesson to carry: a track's script strand can be holding vocabulary in
escrow. HL-C358 said to re-read every uncovered NOTE for a head start; this says
to also grep the WRITING lessons, because a component lesson that teaches a
numeral kanji has taught a numeral whatever its own point was.*

**FINDING TWO: THE TRACK ALREADY HAD A COUNTER, AND ITS LESSON EXPLAINS WHAT A
COUNTER DOES.** `JA-C09-ichido`'s prose reads *"ichi is 'one,' and do counts one
occurrence."* That is **-ど**, the counter 度, taught transparently and never
named as a counter. `JA-A1-NUM-02` says "Nothing in the track touches it." One
thing does. It does not close the point — one counter is not the system, and the
choice between the two series is the hard part — but a tranche that starts by
naming what the reader already has starts one lesson further along, and the note
now says so.

**THE ORDER, AND WHY IT IS COST RATHER THAN CONSTRUCTION.** The ordinal tranches
in this campaign were ordered by what the language builds. Japanese's cardinals
are not built from each other at all — they are ten borrowed readings — so
construction gives no order. What does give one is **price in signs**, and it is
unusually lopsided: of the ten numerals, **eight need only kana the book had
already taught**, and two do not. Chapter 14 (one to five) therefore costs
**zero** new signs, and chapter 15 spends **ろ** on *roku* and **ゅ** on *jū* — and
puts both at the END, so the reader counts round an audible hole at six for three
lessons and can see exactly what a sign buys when one is finally spent. Sanskrit
skipped six for a different reason (a shared ending); this skips it for a
typographic one, which is a reason no previous track in the column had.

**THE GRAMMAR ATOM IS THE SEAM THE COUNTERS WILL RUN ALONG, AND IT SHIPS WITH ITS
EDGES.** Wiktionary's readings boxes give **し** as the on'yomi of four against
kun'yomi **よん**, and **しち** against **なな** at seven — so at two points the
NATIVE word stands inside a borrowed count and is the one actually used. That is
one atom, introduced at four and held at seven. It would have been easy and wrong
to stop there: **はち** has no second name at all, and **く** has a second reading
that the same box also calls on'yomi. Both edges are taught, in that order, so the
reader gets a rule they can see the boundary of. *A rule you cannot see the edge
of is not a rule you can use* — and the two edge lessons cost nothing, because the
signs were already there.

**REINFORCEMENT: THE ONE TRACK WITH A PERFECT RECORD KEPT IT, AND THE MECHANISM
IS WORTH COPYING.** Japanese was one of only two tracks with **zero**
reinforcement-window misses in any window. Fourteen new lessons made **29**
(atom, window) slots newly judgeable on older atoms — 1 R1, 2 R2, 10 R3, 16 R4 —
and every one is answered, so the count stays **0 → 0**. What made that tractable
was that the exposed slots form a **diagonal**: an atom introduced at old position
*p* becomes R4-judgeable exactly when the track reaches *p+80*, so the newly
exposed R4s arrive one per new lesson in their original order, and so do the R3s.
Lesson *n* of the tranche services the R4 of the atom at old position 36+*n* and
the R3 of the atom at 98+*n*. **The payment schedule is therefore not a search
problem; it is arithmetic**, and writing it out first meant every warm-up had a
named recall before a word of prose was drafted. `atomsNeverRevisited` also fell
**1 → 0**: the atom that had never been revisited was introduced by the track's
FINAL lesson, and any tranche appended after it fixes that for free.

**A CLOSURE GATE CAUGHT SOMETHING PROSE REVIEW WOULD NOT HAVE.** The chapters
teach ten numbers and can print the kanji for exactly one of them. Three
Wiktionary citations were written as *"Wiktionary: 四"*, *"七"*, *"九"* — link
TEXT, not URLs — and `measureScriptClosure` counts the whole body including
comments, so those three link labels put `scriptClosureViolations` at 4 and
`neverTaughtGlyphs` at 3 on a track that had held both at zero. **A URL is not
prose and a link label is.** Rewritten as *"Wiktionary: the kanji read shi and
yon"*, both metrics return to zero. Nothing about the lessons was wrong; a
citation was.

**TWO NEW SIGNS, AND TWO DIFFERENT STANDARDS OF EVIDENCE FOR THEM.** **ろ** was
OBSERVED: the cited Sirgazil CC0 animation was fetched, expanded to its 26 frames
and read as images, and the start marker never leaves the upper-left origin, so
the run is single and `penLifts` is 0. **ゅ** claims **no independent handwriting
evidence at all**: it reuses ゆ's observed two-run movement, cites the ゆ animation
plus Unicode's name for U+3085, and says in its `variation` field that the size
adaptation is explicit — which is precisely what U+3063 small tsu already does
from つ. *Two glyphs in one tranche, one with its own observation and one
borrowing another's under a named rule, is the honest shape when a script has
small variants.*

**A LEVEL GATE FLIPPED, WITH THE SAME CAVEAT CHINESE AND MARWADI GOT.**
`JA-C14-yon` realizes `SPINE-COUNT-ONE-TO-FIVE`, so Japanese's `reach` moves
pre-A1 → A1 and **no track in the corpus is now below A1**. Everything else the
track holds is still pre-A1 and `attained` has not moved. One node realized is not
a level reached, and `levels.test.ts` now says so for the third time.

**A COVERAGE TOTAL THAT DOES NOT MOVE IS NOT A TRANCHE THAT DID NOTHING.** The
Japanese A1 coverage stayed at **66/179**, because `JA-A1-NUM-01` and
`JA-A1-NG2-01` were already ticked — one on a single numeral inside a phrase, the
other on the vague half of counting. Ten cardinals DEEPEN two ticks rather than
adding one. **This is the counterpart of HL-C353's finding about coverage
columns**: a column tells you what is taught, not what is required, and a total
tells you how many points are green, not how green any of them is. The repair is
the same one: a named test pin that says which atoms close the point, falsifiable
in both directions.

**WHAT THE QUEUE STILL HOLDS FOR JAPANESE, in dependency order.** The **counters**
(`JA-A1-NUM-02`) — the native *hito-/futa-* series against the Sino *ichi-/ni-*
one, and a counter chosen by the shape of the thing counted; the reader has the
Sino series and one unnamed counter, and nothing else. Then **ordinals**
(`JA-A1-NUM-03`), which need a counter to attach to (*dai-* in front, *-me*
behind) and are no longer blocked on the cardinals; its ordering words — *tsugi*,
*saisho*, *saigo* — are a separate absence that wants no numeral at all. Then the
**nine remaining numeral kanji**, which is a script tranche and not a vocabulary
one. **Zero** waits on **れ** and *kyū* waits on **き**, and both are named in the
lessons rather than quietly skipped.
