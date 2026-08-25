### Measured - the strand cannot finish inside 38 chapters

The queue item that produced this lesson assumed the remaining glyphs were a
matter of authoring more lessons. Measuring the track says otherwise:

- After `TA-W18` (chapter 37) only five speaking lessons remain in the corpus,
  at sequences 1120, 1130, 1140, 1150 and 1160. There is room for one more
  writing lesson and no more, whether it is placed among them or after them.
- Chapter 38's atom load goes 6 -> 8, inside the twelve-atom chapter budget, and
  the lesson introduces 2 atoms, inside the three-atom lesson cap.

So the residue stands at thirteen glyphs in chapter 7's numbers alone — **ஏ**,
**ஐ**, **ஒ** and the ten Tamil digits **௧**-**௰** — with no slot left for any of
them. Closing that debt requires a decision this changelog does not make: extend
the Tamil track past chapter 38, or raise the strand's cadence.

A note on the census, because this entry quotes no total for it and earlier
drafts did. The absolute count of used-but-untaught glyphs is entirely a
function of how "taught" is detected, and small choices swing it by several
glyphs: whether a bold span of four code points such as **ஸ்ரீ** counts as
teaching **ஸ**, how far a negation such as "still wait on letters this book has
not taught" scopes, whether the `TA-C*` lessons may teach as well as use. The
figure of 19 written in `continuity.test.ts` during an earlier change does
reproduce, but only under one particular set of those choices — it needs **ஞ**,
**ஸ**, **ஃ** and **ஷ** to count as NOT taught. Within the writing strand those
four occur only in passing: **ஷ** and **ஸ** in `TA-W03`'s borrowed-ligature
aside (**க்ஷ**, **ஸ்ரீ**) and its Wrap-up answer, **ஃ** in the same lesson's
character-count mention, **ஞ** in `TA-W07`'s sound-table cell **ஞ்ச**. Two of
the four also appear in speaking lessons — **ஞ** in **ஞாயிறு**, **மஞ்சள்** and
**தஞ்சாவூர்**, **ஸ** in **நமஸ்காரம்** — which are uses, not teaching, under
either detector; **ஃ** and **ஷ** appear nowhere in the corpus but that one
`TA-W03` aside.
The detector used here counts all four as taught. Neither reading is wrong; the
number is simply not portable, which is why this entry quotes no total of its
own.

The two facts this entry rests on hold under a detector that does two specific
things, and it is worth naming them rather than claiming detector-independence:
it must scope negation, and it must not treat a `TA-C*` lesson as teaching. Both
matter, and this very lesson is why the first one does — it prints the numbers
it cannot yet spell in bold inside a sentence saying they wait on letters the
book has not taught, so a detector that ignores negation scores those letters
as taught here and puts this lesson's delta at four glyphs instead of one.
(The chapter-39 entry above narrows that sentence, once **ஒ** is taught.)
Chapter 7's own lessons bold the same letters while merely using them, which is
why the second matters. Under a detector that does both: the difference **this**
lesson makes is exactly **ூ**, and the thirteen chapter-7 glyphs named above are
untaught.

This supersedes, rather than continues, the census table in the "last two
glyphs" entry below. That entry's detector scored **ஞ**, **ஸ**, **ஃ** and **ஷ**
as untaught, which is where its 19 came from; the detector described here scores
all four as taught. Both entries then say "thirteen of chapter 7's", and they do
not mean the same thirteen: the earlier list is **ஐ**, **ஒ**, **ூ** plus the ten
digits, this one is **ஏ**, **ஐ**, **ஒ** plus the ten digits. **ூ** moved out of
the untaught set, which is precisely what this lesson did; **ஏ** was in it all
along and the earlier list omitted it. That table is left as written, as a
record of what was measured then.

