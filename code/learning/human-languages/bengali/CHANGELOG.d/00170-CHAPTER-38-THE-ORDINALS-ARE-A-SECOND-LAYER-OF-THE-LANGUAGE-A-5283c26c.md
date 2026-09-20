## Chapter 38 — the ordinals are a second layer of the language, and chapter twelve had already said so

`BN-A1-Q-04` read "Untaught. No ordinal exists in the corpus, so nothing can be
put in a sequence — not a floor, not a date, not a turn." It is now covered:
seven lessons, nine atoms, and one new letter.

**THE NOTE SAID THE MACHINERY WAS UNAVAILABLE. THE CORPUS SAID OTHERWISE.**
Bengali's ordinals are Sanskrit *tatsama* — borrowed back whole — so none of the
"see the number inside it" reading that Gujarati, Marathi and Kannada used is
open here. But **BN-C06-numbers-1-5 had already taught the reason**: it says in
so many words that the old *dv-* is gone from the everyday numeral and is
"alive in words borrowed straight back out of Sanskrit", and it gives **দ্বার**
*dvār* as its example. **দ্বিতীয়** is another of those words. The chapter
therefore opens on **SECOND**, and the thing already present in another guise is
not a word but a **relationship** — a prediction the reader was handed
twenty-six chapters early, coming true.

**THE ORDER, AND WHY IT IS NOT NUMERICAL.**

- **দ্বিতীয়** first: the one ordinal whose relation to its cardinal the reader
  has already been taught. Its lesson also has to say that **the cluster is on
  the page and not in the mouth** — Wiktionary gives the pronunciation as
  *ditiyô*, and a book that romanized it *dvitīyô* would be lying to the reader
  about a word it is teaching them to say.
- **তৃতীয়** second: *dvitīya* : *tṛtīya*, one Sanskrit ending, and now the
  **-তীয়** shape has two members. The atom is introduced HERE and not on
  দ্বিতীয়, because one word is not a shape.
- **চতুর্থ** third: a second ending, **-র্থ**, with one member. Named, and
  explicitly NOT called a pattern — the point it makes is that the borrowed
  layer arrived with more than one shape in it.
- **প্রথম** fourth: no number in it at all. And the finding is that **Bengali
  did not do that** — Sanskrit's *one* was *eka* and its *first* was *prathama*,
  and the two never shared a sound. The gap is older than the language being
  learned. Kannada's rule, reached by a different road: coming fourth, after a
  shape has held twice and a second ending has appeared once, the word can be
  read for what it is instead of taken as a Bengali oddity.
- **ঞ**, then **পঞ্চম** last, because the fifth is the payoff.

**THE PAYOFF IS THAT THE TWO LAYERS ARE VISIBLY ONE WORD.** Samsad's
*Bengali-English Dictionary* keeps **পঞ্চ** *ponchô* as a Bengali headword
glossed **five**, with **পঞ্চম** under it as *fifth*. Set that beside **পাঁচ**
and the two differ by exactly one thing: where the borrowed word has the letter
**ঞ**, the everyday word has the **ঁ** that chapter 12 introduced as "the
chandrabindu, nasalising the vowel". Wiktionary's entry for **পাঁচ** writes its
Sanskrit source as **পঞ্চ** and walks the road — *pañca*, *paṃca*, a Middle
Bengali form already spelled with the mark, **পাঁচ**. So the chapter's thesis is
checkable rather than asserted, and the evidence is four letters wide.

**THE ONE LETTER, TAUGHT THE WAY HL-C212 REQUIRES.** পঞ্চম needs **ঞ**, which
was shown in the corpus and taught nowhere. Commons holds a stroke-order
animation for every Bengali independent vowel and for **no consonant at all**,
so — following BN-W05-tha and BN-W05-tta exactly — the lesson teaches place of
articulation and the row **ঞ** completes, cites the Unicode Bengali chart, and
**claims no pen path**, because none can be sourced. `neverTaughtGlyphs` falls
**9 → 8**.

**NO AGREEMENT, AND THE CHAPTER SAYS SO.** A Bengali ordinal stands in front of
its noun and never changes, exactly like **লাল**. That is worth one sentence and
no more, and it is the opposite of what the Gujarati and Marathi ordinal
chapters had to teach.

**REINFORCEMENT, DECOMPOSED ATOM BY ATOM RATHER THAN BY TOTALS:**

    reinforcementWindowMisses        249 -> 242
    reinforcementMissesByWindow-R1    38 ->  38
    reinforcementMissesByWindow-R2    49 ->  49
    reinforcementMissesByWindow-R3    90 ->  90
    reinforcementMissesByWindow-R4    72 ->  65
    atomsTaught                      197 -> 206
    atomsNeverRevisited                3 ->   3
    neverTaughtGlyphs                  9 ->   8

The tranche's own nine atoms create **zero** debt. Seven more lessons make **17**
(atom, window) slots newly judgeable on OLDER atoms — three R2, six R3, eight R4
— debt the length EXPOSES rather than creates; each was assigned by window
arithmetic to the exact new lesson inside its window and **all seventeen are
answered**. Seven pre-existing R4 misses close outright, and they are exactly the
material this chapter had a reason to reach for: the five numbers, the two
chapter-12 histories under দুই, the chandrabindu, and the three letters of the
চ row that **ঞ** completes. Zero regressions on previously judged slots.

**THE RECALL LINES ARE IN THE PROSE, NOT ONLY IN THE FRONTMATTER.** Every
warm-up names its recalls by word, and the declared `assesses` list is exactly
those; the blocks were audited one at a time against the paragraphs beside them,
and four lists that named an atom the prose did not engage were corrected rather
than left to make the metric look tidy.

**THE SCRIPT COST WAS MEASURED, NOT ASSUMED.** The taught set was DERIVED from
the corpus at point of use — every Bengali codepoint in the headword of every
`BN-W*` lesson, 36 of them before this chapter and 37 after — and every Bengali
character in the tranche is in it. `scriptClosureViolations` holds at 21, all of
it pre-existing. Every Latin diacritic used was checked against
`core/main-font-charset.json`.

**NOT CLAIMED.** The ordinals past fifth (Bengali keeps borrowing them, but each
one is a separate word and none of them is derivable from a cardinal the reader
has). The Bengali digits, still zero of ten. And `BN-A1-Q-04` closed **without
any of the cardinals moving**, which is worth recording: the ordinal column here
never depended on the count going past five, because none of these words is
built on a count.

**VERIFIED.** `npm run build`, then all ten `check:*` gates from `dist`; the
whole package suite; language-ladder via `bash BUILD`; and the Bengali book
compiled with XeLaTeX through the materialized entrypoint, with the new pages
read as rendered images rather than through `pdftotext`.

