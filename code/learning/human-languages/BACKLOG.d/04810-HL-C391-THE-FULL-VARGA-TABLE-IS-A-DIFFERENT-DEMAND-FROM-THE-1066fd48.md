## HL-C391 — the full varga table is a different demand from the one TE-A1-L-10 makes

`TE-A1-L-10` is now closed. Its label asks for **"the consonant letters, as a
set a reader can finish a page with,"** and after `TE-S157` through `TE-S160`
every consonant the Telugu corpus prints anywhere has a recognition lesson.
This entry exists so nobody reads that closure as a claim the track teaches the
whole alphabet, and so the point is not quietly reopened instead of a new one
being written.

### What is taught and what is not

Thirty-one of Telugu's thirty-six consonant letters have a lesson. The five
without one are:

| letter | romanized | uses across all 365 lesson bodies |
|---|---|---|
| ఙ | *ṅa* | 0 |
| ఛ | *cha* | 0 |
| ఝ | *jha* | 0 |
| ఱ | *ṟa* | 0 |
| ఴ | *ḷḷa* | 0 |

The first four are letters of the modern alphabet. `ఴ` is archaic and is not in
`data/scripts/telugu.json` at all, which is the `HL-C385` status.

`U+0C29` sits inside the same Unicode block and is **not a letter**: it is an
unassigned code point. An earlier count treated it as missing script debt.
Iterating a Unicode block is not the same as listing a script's letters.

### Why they were not taught in this pass

Every script lesson in this track anchors its character in a word the reader
already says — `మ` in నమస్కారం, `ఠ` in జ్యేష్ఠం, `ఫ` in ఫాల్గుణం. None of the
five has such a word, because none of the five occurs anywhere in the corpus.
Teaching them would mean inventing an anchor, and a made-up anchor is worse
than an honest gap.

### What the new point would need

A point demanding the full varga table — five rows of five, plus the semivowels
and sibilants, recited as a table — is a different skill from recognising a
letter inside a word, and it needs:

1. Words that actually use ఛ, ఝ and ఱ, landed in the corpus first. (ఙ occurs in
   modern Telugu essentially only inside conjuncts, so it needs the conjunct
   strand, not a letter lesson.)
2. A decision on `ఴ`, which is archaic; teaching it at A1 is hard to defend.
3. Its own inventory entry under `Telugu lipi`, with a probe naming the
   recitation atom rather than the recognition atoms.

Write it as `TE-A1-L-NN`. Do not reopen `TE-A1-L-10`: its label is satisfied,
and rewriting a closed point's meaning to absorb a larger demand is how a
coverage number stops meaning anything.
