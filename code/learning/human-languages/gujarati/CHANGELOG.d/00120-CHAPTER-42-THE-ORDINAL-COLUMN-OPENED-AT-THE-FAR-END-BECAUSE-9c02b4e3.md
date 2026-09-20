## Chapter 42 — the ordinal column, opened at the far end because the dictionary said where the rule was

`GU-A1-NUM-05` read "Not taught." It is now covered, and the chapter that closes
it is six lessons and eight atoms.

**THE ORDER CAME OUT OF A SUFFIX ENTRY, NOT OUT OF THE NUMBERS.** Wiktionary's
entry for **-મું** says the suffix builds the ordinal of every Gujarati number
*except* એક, બે, ત્રણ, ચાર and છ. The track's cardinals stop at પાંચ. Lay one
fact on the other and the reachable set is **four exceptions and exactly one
rule** — so the chapter opens on **પાંચમું**, which Wiktionary itself analyses
as પાંચ + -મું while giving the other four an inheritance instead. Kannada's
reason in Gujarati's terms: a broken shape is unreadable until the shape has
been seen. The lopsided ratio is a fact about where the count stops, not about
Gujarati, and the payoff says so on the page.

**BETWEEN THE FOUR EXCEPTIONS, THE ORDER IS HOW MUCH OF THE NUMBER SURVIVES.**
ત્રીજું keeps ત્ર whole, r and all — the r chapter 12 spent a lesson on.
બીજું keeps only the **b**, which is the reader's own chapter-12 finding
(દ્વે → *dv* → *bb* → *b*) and nothing else. ચોથું keeps the ચ. પહેલું keeps
nothing of એક at all, and lands last because its story is only legible once
પાંચમું has been held for four lessons.

**NOTHING WAS ALREADY PRESENT IN ANOTHER GUISE, AND THAT WAS CHECKED RATHER THAN
ASSUMED.** HL-C358 said to re-read an "Untaught" note for a head start, and named
Gujarati as a track to read for a *pehlā*-shaped word before pricing it. There is
none: no ordinal, and no discourse or pairing word carrying one, appears anywhere
in the 263 lessons. Marathi's and Tamil's move was unavailable here, so the
chapter teaches five new words rather than four.

**TWO SHAPES ARE CLAIMED AND A THIRD IS NOT.** બીજું and ત્રીજું share **-ીજું**
from two Sanskrit words that shared *-tīya* (*dvitīya*, *tṛtīya*), and that atom
is introduced on બીજું rather than on ત્રીજું because one word is not a shape.
ચોથું ends in **થ** from *caturthá* and has no fellow in the reachable count, so
the book names it and declines to call it a pattern.

**THE PAYOFF IS THAT FIRST AND FIFTH SET OUT FROM THE SAME SANSKRIT ENDING.**
*prathamá* and *pañcama* both end in *-ma*; Wiktionary traces **-મું** to
*-maka*, built on that *-ma*. Prakrit turned *prathama*'s into *-ll-*
(*prathilla* → *pahila*), and the **લ** of પહેલું stands where it was. The one
ordinal with no number left in it is the one that began with the very ending
taught first. Chapter 12 showed a sound lost and restored; this is the same
history running the other way.

**REINFORCEMENT, DECOMPOSED ATOM BY ATOM RATHER THAN BY TOTALS:**

    reinforcementWindowMisses        364 -> 362
    reinforcementMissesByWindow-R1    29 ->  29
    reinforcementMissesByWindow-R2   124 -> 124
    reinforcementMissesByWindow-R3   149 -> 149
    reinforcementMissesByWindow-R4    62 ->  60
    atomsTaught                      257 -> 265
    atomsNeverRevisited                6 ->   4

The tranche's own eight atoms create **zero** debt: each lesson recalls the
previous lesson's ordinal (R1) and the payoff services the first lesson's R2,
which is every window a 269-lesson track is long enough to judge for atoms
introduced at positions 263-267. Six more lessons make **15** (atom, window)
slots newly judgeable on OLDER atoms — one R1, five R2, six R3, three R4 — debt
the length EXPOSES rather than creates; each was assigned by window arithmetic
to the exact new lesson inside its window and **all fifteen are answered**. Two
pre-existing R4 misses close outright: મહિનો and અત્યારે are said and written
again at distance 80, the first time either has been inside its fourth window.
કેટલા and કેમ?…કેમકે…, which no lesson had ever revisited, are revisited now.

**THE RECALL LINES ARE IN THE PROSE, NOT ONLY IN THE FRONTMATTER.** Every warm-up
names its recalls by word and the declared `assesses` list is exactly those; the
blocks were audited one at a time against the prose beside them, and five block
lists that named an atom the paragraph did not actually engage were corrected
rather than left to make the metric look tidy.

**THE SCRIPT COST NOTHING, AND THAT WAS MEASURED RATHER THAN ASSERTED.** The
taught set was derived from the corpus at point of use — every Gujarati codepoint
in the headword of every `GU-W*` writing lesson, 44 of them — and every Gujarati
character in the tranche, headwords and examples alike, is in it.
`scriptClosureViolations` 0 and `neverTaughtGlyphs` 0, both unchanged. Sanskrit
and Prakrit forms are printed in romanization only, following the precedent
`GU-C06-number-histories` already set, so no Devanagari enters a Gujarati book.

**NOT CLAIMED.** The ordinals past fifth: the ending keeps working and the
chapter says so, but listing સાતમું or આઠમું would print a cardinal the track has
not taught. That work is `GU-A1-NUM-03` (cardinals six to ten), which is now the
single blocker for the rest of the column — as it is for the Gujarati digits,
which remain zero of ten (`GU-A1-NUM-08`, `GU-A1-SCR-11`).

**VERIFIED.** `npm run build`, then all ten `check:*` gates from `dist`; the
whole package suite, 136 test files / 1951 tests passed; language-ladder via
`bash BUILD` (39 test files / 442 tests passed); and the Gujarati book compiled
with XeLaTeX through the materialized entrypoint — exit 0, 359 pages, zero
Missing character, zero overfull, zero underfull — with the new pages read as
rendered images rather than through `pdftotext`.

