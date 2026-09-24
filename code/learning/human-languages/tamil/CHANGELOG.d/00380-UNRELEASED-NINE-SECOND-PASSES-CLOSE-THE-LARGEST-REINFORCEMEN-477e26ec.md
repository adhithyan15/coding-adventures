## Unreleased — nine second passes close the largest reinforcement debt in the corpus

Nine lessons. None of them teaches anything, and none needed a new home.

| metric | before → after |
|---|---|
| pre-A1 atoms revisited fewer than twice | 73 → **0** |
| ladder blockers | 3 → **2** (`vocabulary 208/300`, `atom-budget 1`) |
| lessons | 416 → 425 |
| `atomsNeverRevisited` | 50 → **35** |
| atoms taught | unchanged (504) |
| `forwardReferences` | unchanged (8) |
| atoms introduced | **0** |
| new chapters / segments / extensions | **0** |
| reinforcement-window misses | 1070 → 1082 |

Tamil was the largest of the six reinforcement debts and the last this shape of
tranche answers. Twenty-two of twenty-three tracks now carry none.

### Seventy-three atoms, eighty-eight retrievals

Fifteen of the seventy-three had no later revisit at all, and an atom at zero
costs **two** lessons rather than one — `practisedAtoms` is a set per lesson, so
naming an atom three times on one page still earns it a single revisit. That is
`(15 x 2) + 58` = eighty-eight slots against seventy-three atoms.

Eighty-eight is the biggest number in the programme and the tranche was still
cheap, because **slot count and cost are different questions**. Forty-two of the
seventy-three sat on two segments, and chapters 84-86 sit at the far end of a
track whose debt lives in chapters 1-39 — maximum retrieval distance, on
segments that already existed.

### Where the nine went

| chapter | lessons | what they retrieve |
|---|---|---|
| 84 | 4 | the 25 script pieces, then the 18 reading companions |
| 85 | 3 | the courtesies, the name exchange, the farewells and first verbs |
| 86 | 2 | the four late leftovers, then the fifteen that came once |

### The script pages sort by ear, not by date

**Tamil writes three n-letters** — **ந** dental, **ண** retroflex, **ன**
alveolar — for what English hears as one sound. They arrived one per chapter,
pages apart, and no page had put them side by side. The same is true of **ல**
and **ழ**, and of **ர** and **ற**. Sorting by what a reader confuses rather
than by when it arrived is the lesson.

**The u-family turns out to be a square.** Standing **உ** and **ஊ** against
hanging **ு** and **ூ** — four corners that arrived in chapters 17, 36, 37 and
38. No chapter could draw the grid because no chapter held all four. **உணவு**
holds two corners in one word: standing at the front, hanging at the end.

And three separate chapters each touched one idea once without naming it: **a
vowel sign can be written on the wrong side of its consonant.** இல்லை and பெயர்
put one to the left; **ோ** is built from two marks, one on each side. The order
on the page is not the order in the mouth.

### The pages that are not about script

- **chapter 85 — the courtesies.** **வணக்கம்** and **நன்றி** are both native
  where neighbouring languages borrowed from Sanskrit, and **நன்றி** carries a
  warning: between people who are close, saying thanks out loud can sound
  *marked*. Also that **பரவாயில்லை** covers both *no problem* and *you're
  welcome*, which English keeps apart.
- **chapter 85 — the name exchange.** **என் பெயர் …** has no word where English
  puts *is*, and neither does the question. Tamil does not need a verb to link
  two things.
- **chapter 85 — going and doing.** Read the literal column down and **Tamil has
  no word for goodbye**: *I will go and come back*, *we will meet once more*,
  *tomorrow, let us see*. Not one of the three names the parting.
- **chapter 86 — who is in the room.** **நாம்** takes the listener in and
  **நாங்கள்** shuts them out, audibly, in front of them. Beside it, **இவர் என்
  நண்பர்** is verbless too — the same silence as the name sentence, twenty
  chapters later and never linked until now.

### The gate that nearly bit

`forwardReferences` sits at **8 against a ceiling of 8**, and `lessonsEarly > 1`
at **4 against 4**. Both were exactly met before this tranche, so one page
printing a word the course teaches later would have failed the build. The test
that asserts it is named for the opening chapters; two of its three assertions
are whole-track ceilings.

Hosting in chapters 84-86 of an 86-chapter track left only eight words that
count as "later". The last page names **நாம்** and **நாங்கள்**, which are two of
them — taught at chapter 85, used at chapter 86, therefore backward and free.
Confirmed by running the count rather than reasoning about it. It held at 8 and
4.

### The one metric that moved the wrong way

Reinforcement-window misses go 1070 → 1082. That is the structural cost of
appending at the end of a corpus: nine lessons that do not revisit an older atom
widen every open window they fall inside, and atoms introduced in the last
chapters have nothing after them. Hindi's chapter 105 tranche paid the same toll.
Recorded rather than explained away.

Curriculum digest `998a8193`/7555 → `297f88ff`/7564 on a **30-line** structural
diff.
