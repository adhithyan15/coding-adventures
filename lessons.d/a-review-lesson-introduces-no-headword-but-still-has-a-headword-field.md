---
category: Testing & coverage
---

# A review lesson introduces no headword but still has a headword FIELD, and a census that reads fields will move

Six tranches of retrieval-only lessons had established a reliable sentence: *`review` is outside
`CONTENT_TYPES`, so these lessons introduce no headword and no vocabulary count moves.* True every
time, and it made the pin updates mechanical — lesson count up, everything else held.

The seventh track had a census that counts something else. Its exam-inventory test measures, per
glyph, how many lesson **fields** contain it and how many distinct **tokens** do:

```ts
const headwords = lessons
  .map((lesson) => lesson.realization.headword ?? "")
  .filter((headword) => headword.includes(glyph));
const tokens = new Set(headwords.flatMap((h) => h.split(/\s+/).filter((t) => t.includes(glyph))));
return { glyph, fields: headwords.length, tokens: tokens.size };
```

A `review` lesson **has** a `headword:` in its frontmatter. It is a label for what the lesson
retrieves — the glyphs being recalled, the phrase being run cold — not a word the lesson teaches. The
level gate ignores it because the lesson's *type* is not a content type. This census does not ask
about type at all; it reads the field.

So no token count moved, and three field counts did — one for each review lesson whose label happened
to contain a glyph the census tracks.

**"Introduces no headword" and "contributes no headword field" are different claims.** The first is
about the atom graph; the second is about the frontmatter. A gate keyed on type sees only the first.
Any census, count, or report that reads `realization.headword` directly sees the second, and there is
no reason those two should agree.

**A second, smaller trap rode along.** One glyph's field count rose into a tie with another, and the
comparator broke ties on `localeCompare`, so the two swapped places in a sorted pin. A pair reordering
looks like a second count moving; it is the tie-break doing its job. Check the comparator before
concluding anything about the values.

**What made this cost three attempts instead of none** is that I wrote the explanatory comment before
computing the numbers. First draft: *every count below is unchanged*. Second: *two field counts move*.
Third, and correct: three move, and here is the pair that reorders and why.

The file I was editing already contained this exact warning, about a different chapter:

> THE FIRST DRAFT OF THIS COMMENT CLAIMED THE COUNTS BELOW WOULD HOLD, on the reasoning that a chapter
> teaching a CONSTRUCTION out of pieces already owned adds no headword and therefore no glyph. Half of
> that is wrong and the test caught it.

I read that note while editing around it and reproduced the failure anyway, because the reasoning
*felt* sound and the alternative was three lines of node. **Compute the census, then write the
sentence.** Deriving what a set of headwords contains is not something to do in your head when the
data is one command away — and a comment that explains a number is worthless if the number came from
reasoning rather than measurement.

Related: [[a-corpus-count-is-measured-against-a-base-and-pushed-against-a]] — the same discipline from
the other direction: there the number was real but its base went unstated; here the base was fine and
the number was imagined.
