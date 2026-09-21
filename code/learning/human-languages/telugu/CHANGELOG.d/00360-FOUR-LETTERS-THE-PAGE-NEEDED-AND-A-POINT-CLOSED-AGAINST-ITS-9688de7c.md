## Four letters the page needed, and a point closed against its own label

`TE-A1-L-10` — *the consonant letters, as a set a reader can finish a page
with* — closes. Telugu A1 coverage **216/326 → 217/326**; the script column
goes **13/23 → 14/23**.

| metric | before → after |
|---|---|
| exam-point coverage | 216/326 → **217/326 (67%)** |
| characters used but untaught | **9 → 5** |
| uses of an untaught character | **898 → 489** |
| atoms taught | 470 → 474 |
| measurable lessons | 360 → 365 |
| `forwardReferences` | unchanged |
| `durationViolations` | unchanged (0) |
| `atomsNeverRevisited` | unchanged |
| `payoffSurprises` | unchanged (0) |
| `scriptClosureViolations` | 9 → 8 |
| reinforcement-window misses | 864 → 879 |

### What was wrong

Four consonants were printed on this track's pages with nothing teaching them.
Counted over every Telugu lesson body before this pass:

> బ 185 · శ 136 · ష 86 · ఫ 4

**బ opens బాగా**, the word this book answers *"how are you?"* with, in chapter
three. **శ opens two of the seven day names**, శుక్రవారం and శనివారం, in a
chapter whose whole job is reciting that list. **ష closes సంతోషం**, taught in
chapter two: స at that word's other end gets its lesson in chapter five, and
until this pass ష got none at all, so a reader reached chapter five able to
read one end of the word and not the other.

### What was added

| lesson | seq | ch | character | anchored in |
|---|---|---|---|---|
| `TE-S157-letter-ba` | 181 | 3 | బ | బాగా / బాగున్నాను |
| `TE-S158-letter-ssa` | 276 | 5 | ష | సంతోషం |
| `TE-S159-letter-sha` | 371 | 10 | శ | శుక్రవారం / శనివారం |
| `TE-S160-letter-pha` | 445 | 17 | ఫ | ఫాల్గుణం |
| `TE-S161-script-recall-four` | 446 | 17 | — | cold retrieval of all four |

**Not one of the four is taught as a shape on its own.** ష and శ are taught
against స, which was already known, as the three s-letters Telugu keeps apart
and Tamil's own alphabet has none of. ఫ is taught against ప exactly as ఠ was
taught against ట a page earlier — one position, with and without the breath.
బ is taught against the fact that Tamil writes no separate letter for the
sound at all.

Placement follows this track's design: each letter sits after the word that
first needs it, not before. ష is the one exception and the reason is
deliberate — its anchor word is in chapter two, but స is not taught until
chapter five, and teaching the curled sibilant before the plain one would put
the contrast in the wrong order. It sits one slot after స instead.

### Why the point closed, and what it does not claim

Thirty-one of Telugu's thirty-six consonant letters now have a lesson. The five
without one — ఙ, ఛ, ఝ, ఱ, ఴ — have **zero occurrences across all 365 lesson
bodies**. Every consonant the corpus prints anywhere is now taught, which is
precisely what this point's label asks for.

The closure is against the label and **not** against the full varga table.
A point demanding all thirty-six is a different demand and needs its own
inventory entry; `BACKLOG.d` `HL-C391` writes it down, including why the five
cannot be taught the way this track teaches a letter — every script lesson here
anchors on a word the reader already says, and none of the five has one.

`U+0C29` is not among the five because it is not a letter: it is an unassigned
code point that an earlier count mistook for missing script debt.

### What is left

The five characters still used-but-untaught are all **independent vowels** —
అ 301, ఆ 104, ఏ 57, ఐ 13, ఒ 12 — and all of them are `TE-A1-L-08`'s business.
That point is left open, and its note now says so.

