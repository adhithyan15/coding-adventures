### Removed — six more lessons stop teaching script

`TA-C02-nii-niingal` and all five chapter 3 lessons drop their `## The letters in this
word` sections, and `ch02-introductions.tex` and `ch03-responding.tex` drop the four
matching `sounds` boxes. Verified against the **generated** manifest rather than the
source: all six flip `reasons: ["script-block"]` → `["no-visual-dependency"]` and their
`detachableSegments` lists empty, so the script teaching genuinely left the lesson.

Two of the six sections were not letter lessons at all. `TA-C03-eppadi-irukkirirgal`
used the heading to carry **verb morphology**, which is speaking content that happens to
be printed in Tamil; it moves into the lesson's own prose as *iru* + *-kkiṟ-* +
*-īrgaḷ* — the segmentation `TA-C32-iru` already pins, and one that concatenates to the
surface word — rather than being deleted. `TA-C03-paravayillai`'s section was a
word-joining note the very next section already makes, so it goes.

`sounds:` frontmatter is **kept** on all six, unlike the chapter 2 pass. Those lessons
list genuine pronunciation ids (`long-aa`, `retroflex-kk`, `final-m`); chapter 2's
listed script ids (`independent-e`, `pulli`), which is why clearing them was right
there and would be wrong here.

Chapter 3 is closed; chapters 4-5 are not. The claim carried over from the last pass —
that their boxes show ப, எ and ய — was wrong, so here is the measured inventory of the
four `sounds` boxes in `ch04-farewells.tex` and `ch05-first-verbs.tex`, by the chapter
that first teaches each glyph:

| | glyphs |
|---|---|
| never taught by any strand lesson | **ோ**, **ே**, **ழ** |
| taught long after book chapter 5 | **ா** (25), **ள** (27), **ு** (31), **ப** (23), **ர** (19), **ச** (19), **ல** (18), **ை** (18), **்** (7), **ந** (6), **ம** (6) |
| taught in time | **வ** (4), **க** (5) |

So the debt there is bigger than "three glyphs," and it is two different debts: three
glyphs the strand never reaches at all, and eleven it reaches only much later. Neither
is addressed here.

