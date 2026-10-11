## HL-C453-1b64daca — the Kannada and Tamil script-recognition segments sit before their anchor words in curriculum path order

**Status: OPEN — investigated, deliberately not moved.** A review noticed that
KA-S111-vowel-sign-e (anchor word ಹೆಸರು) and TA-S101-letter-pa (anchor word
பெயர்) come before the lessons that teach their anchor words in curriculum path
order. Both letter lessons are right in **reading order**, which is chapter and
then `sequence:`. The book, the narration and `measureContinuity` all use that
order. KA-S111 is chapter 2, sequence 125, after KA-C02-hesaru (70) and
KA-C02-nimma-hesaru-enu (120). TA-S101 is chapter 6, sequence 411, four chapters
after TA-C02-peyar.

**The two lessons are not special.** In each track, every script-recognition
lesson sits in one segment, `KA-PATH-100` / `TA-PATH-100`. That segment is
`curriculum.d/path/0040-*`, so it comes after `PATH-003` and before the segment
that teaches the chapter 2 names. Matching each letter lesson's "You already say
these" words against the headwords of other lessons gives:

- **Kannada:** 29 of the 45 script lessons that list anchor words have at least
  one anchor word taught later in path order. None has one taught later in
  reading order.
- **Tamil:** all 15 such lessons have at least one anchor word taught later in
  path order. In reading order, none comes before a lesson that teaches its
  anchor word.

**Why moving only these two would not help, and could break things.**

1. The validator requires an extension to attach to exactly one segment, and
   it rejects an extension lesson placed outside that segment ("uses X outside
   Y"). Each segment's `spine_node` must also match the `spine_node` of every
   lesson in it. Moving one letter therefore means a new extension node, a new
   spine node for that letter, and renumbered `curriculum.d` shards.
2. That work would fix two lessons out of 44, and the pattern it leaves behind
   would look deliberate.
3. Neither lesson declares `prerequisites`, so the path-order prerequisite check
   (`curriculum-prerequisite-order`) has nothing to judge. The book and the
   narration print in reading order, not path order.

**What the real fix is.** Treat path order as a delivery order. Then either
split each track's `PATH-100` segment so that each script lesson sits in or
after the segment of its first anchor word, or attach the script extension
`after` a later segment. Do it for the whole segment in one change. Then give
each letter lesson its anchor word's lesson as a `prerequisite`, so that
`validate` enforces the order from then on. Script closure, the glyph budget,
the chapter gates and the curriculum-shard tests all need to stay green.
