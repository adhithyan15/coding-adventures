## HL-C313 — R2 measured on every track, and the chapter-boundary reach that closes it

The earlier HL-C313 entry inferred from Hindi that "Tamil's, Kannada's and
Marathi's tranches are built to the same shape and will measure the same way".
Two of those three guesses were right and one was wrong, which is why this entry
exists: the shape is now measured on all 23 tracks rather than assumed, and
Marathi turns out to have **zero** R2 debt of this kind.

### The mechanism, confirmed rather than restated

Inside a chapter of five one-word lessons at reading positions `p .. p+4`, the
k-th lesson practises the chapter's first k words. Word *i* is therefore
retrieved at distances `1 .. 5-i`, and the next chapter's opening lesson reaches
back for the last two at distances 1 and 2. **The largest retrieval distance the
shape can produce is 4.** R2 is 5-15, so it is not merely missed, it is
unreachable.

The corpus contains its own control. Telugu chapter 51 has two script lessons
interleaved among its five words; those two extra lessons push the first word's
retrieval from distance 4 out to 6, and that word — alone in chapters 46-73 —
was already inside R2 before this work. Insert two lessons and the defect
disappears; that is the mechanism and not a coincidence of authoring.

Distance histograms make the cliff visible. Practice events by distance:

    hindi     d1:244 d2:202 d3:95 d4:61 | d5:20 d6:9  d7:11 d8:13
    sanskrit  d1:203 d2:178 d3:114 d4:44 | d5:20 d6:43 d7:22 d8:10
    marwadi   d1:215 d2:143 d3:113 d4:92 | d5:140 d6:81 d7:56 ... d20:131
    japanese  d1:102 d2:31  d3:18  d4:17 | d5:74  d6:18 d7:12 ... d20:30

Marwadi and Japanese have deliberate humps at 5 and at 20 and score **zero
misses in every window**. They are the existence proof that the corpus already
knows how to schedule this; the tranche shape simply does not.

### R2 misses per track, measured with `measureContinuity` over the merged tree

`one-per-lesson` counts R2-missing atoms whose introducing chapter has four or
more lessons and introduces exactly one atom in every one of them — the shape
this entry is about. `other` is R2 debt of some different shape, which this fix
does not touch and should not be blamed for.

    track        chapters    R2  one-per-lesson  other  affected chapters
    spanish           417  1151             533    618        110
    sanskrit           51   279             180     99         37
    hindi              81   355             172    183         35
    tamil              81   336             166    170         34
    kannada            73   302             131    171         27
    malayalam          66   278             122    156         25
    bengali            26    85              60     25         16
    punjabi            36   113              34     79          8
    telugu (before)    80   301             163    138         33
    telugu (after)     80   162              24    138          6
    chinese            18    55              21     34          5
    gujarati           34    98              21     77          4
    latin              47   163              16    147          4
    french             33   140               4    136          1
    russian            15   101               3     98          1
    arabic             36   165               0    165          0
    german             36   206               0    206          0
    italian            25   148               0    148          0
    marathi            36    93               0     93          0
    persian            15    99               0     99          0
    portuguese         26   158               0    158          0
    urdu               18   108               0    108          0
    japanese           13     0               0      0          0
    marwadi            31     0               0      0          0

Corpus R2 went 4734 -> 4595 with Telugu, and the attributable share 1626 ->
1487. **1487 of the remaining 4595 misses are this shape**; the other 3108 are
not, and a plan that treats every R2 miss as the tranche defect would be wrong
about two thirds of them.

### The rule that shipped, on Telugu chapters 48-75

Each of a chapter's five lessons carries one extra `[YOU RECALL: ...]` task
naming the word at the **same position two chapters back** (distance 10, dead
centre of R2) and the word at the same position **one chapter back**
(distance 5). Two arms rather than one: distance 5 sits on the window edge and
survives no drift, distance 10 survives five lessons of it either way, and every
atom therefore gets two independent hits. The generator asserts each computed
distance is inside 5-15 against the *actual* reading order and refuses to write
otherwise, so a chapter with an interleaved script lesson cannot silently drift
out of the window.

Cost is one bullet and one frontmatter atom pair per lesson. The 300-second
ceiling is computed from content, so this is real budget: the median computed
duration of Telugu chapters 46-80 moved 127s -> 133s and nothing came near the
ceiling. No lesson was split, added, removed, or reseated.

This is deliberately NOT the fix the earlier entry proposed (third lesson
retrieves the previous chapter's first two words, fifth retrieves its third).
That version reaches distances 6-8 and covers three of five words; this one
covers five of five and does not need the author to remember which lesson is the
third.

### The plan for the rest, in the order the numbers argue for

1. **Sanskrit (180), Hindi (172), Tamil (166)** — the same shape, the same
   Indic vocabulary tranches, roughly 35 chapters each. One PR per track. Hindi
   and Sanskrit are the two tracks where this was independently found, so they
   also serve as the confirmation that the Telugu result generalises.
2. **Kannada (131), Malayalam (122), Bengali (60)** — same, smaller.
3. **Punjabi (34), Telugu remainder (24), Chinese (21), Gujarati (21),
   Latin (16)** — worth one combined pass, not five PRs.
4. **Spanish (533 across 110 chapters)** — the largest by far and the one to do
   last, not first. Spanish is under an active drive loop with several branches
   in flight; 550 lesson edits landing at once would conflict with all of them.
   Split it by chapter band and land it behind the Indic tracks.
5. **French (4) and Russian (3)** — not worth a PR of their own; fold into
   whatever else touches those tracks. French chapters 9-16 are mid-migration
   and must be left alone until that lands.
6. **Arabic, German, Italian, Marathi, Persian, Portuguese, Urdu — do not apply
   this fix.** Their R2 debt is entirely of some other shape (0 attributable
   atoms between them) and a chapter-boundary reach would add retrieval lines
   without moving the number. They need their own reading.

Two things that are NOT in scope and should not be smuggled in. The window
definitions in `continuity.ts` are not to be widened; R2 is reachable, as
Telugu now demonstrates. And `continuity.ts` already skips a window a track is
too short to contain (`at + window.from > last`), so a short track missing R4 is
not a defect and must not be "fixed".
