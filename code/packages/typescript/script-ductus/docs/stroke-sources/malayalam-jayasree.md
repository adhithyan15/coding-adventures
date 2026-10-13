# A recording as a source, with a licence that asks for credit (Malayalam ് ഠ ൊ ോ ൧-൯)

The candrakkala ്, the consonant ഠ, the digits ൧-൯ and the two-part vowel
signs ൊ and ോ (`strokes/malayalam.ts`, after ൈ) cite Jayasree
(`sachn1/jayasree`, Sachin Nandakumar), a Malayalam handwriting animator built
on about 300 centre lines that one recorder traced over the Manjari typeface,
one gesture per pen-down stroke. Because it is a recording, it shows lifts
directly, not only order and direction; every glyph here is one recorded
stroke except ൊ and ോ, which are two: the left sign, a lift, then ാ. Its stroke data is CC BY 4.0, which allows
adaptation with credit, so each record names the work, its author, the commit
and the licence, and the citation printed under each strip carries that
credit. Even so, only the facts are taken (stroke count, start, order,
direction, end): Manjari is rounder and wider than Noto, so every path is
fitted to the bundled Noto Sans Malayalam outline at the default tolerances,
and the captions are this package's own. One recorder, so confidence is
medium. Jayasree breaks the earlier tie on ഠ's direction (anticlockwise, with
Thooval and grahyam, against Moag's clockwise arrow) and supplies the movements
Moag never gave ്.

**The one coverage exception: a consonant placeholder.** Printed alone, ൊ and
ോ still leave room for the consonant they are written around, and Noto marks
it with a small dot between the two parts. The dot shows where a consonant
would sit; nobody writes it, and a path that skips it leaves about 5% of the
glyph untraced, over the 2% limit. So the coverage check skips exactly that
contour for exactly these two glyphs (`NOTO_PLACEHOLDER_CONTOURS` in
`tests/support/stroke-honesty.ts`, keyed by ductus key, each naming the
contour's index and pinned bounds). The limit itself is unchanged, the on-ink
and join checks still see the whole glyph, and the tests prove the skipped
contour is the dot and nothing else: the rest of each glyph is exactly the
cited left sign's outline and ാ's outline shifted. ൦ is recorded but no lesson
draws it.
