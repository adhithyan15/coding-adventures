## HL-C423-d47ac754 — cousinsFor has no production consumer, so 494 cousin panels render nowhere

**Status: OPEN.** Found while measuring the payoff of the HL-C419
normalisation, and recorded because the measurement was nearly reported as a
shipped improvement when it is not one.

`src/cousins.ts` carries the corpus's etymological join. It is careful work: it
joins on exact string equality, deliberately refuses `concept_tag` because that
"would emit false etymology at scale", fixes the cousin language order so book
hashes do not churn, and picks one best cousin per language. `HL-C419` built a
guard, a declared tag vocabulary and a 192-entry debt baseline to protect that
join key, and then normalised 110 splits to repair it.

**Nothing reads it.** A repo-wide search finds `cousinsFor` and
`buildCousinIndex` exported from `index.ts`, exercised by `tests/cousins.test.ts`,
and called by **no production code**. `book.ts` builds its `cousinweb`
environment from an authored `etymology` block in the lesson body — prose a
human wrote — not from the joiner. The Language Ladder app's `siblings.ts` is
about script families, not etymons.

So the measured gain from HL-C419 is real and invisible:

| | before | after |
|---|---:|---:|
| lessons with a cousin panel | 365 | 494 |
| cousin pairs joined | 575 | 866 |

Not one line of book text changed; every regenerated `.tex` differed only in its
canonical-source hash.

### Why this is worth an entry rather than a shrug

The same reasoning HL-C419 applies to a missing panel applies here one level up.
*"A missing cousin panel looks exactly like a word that has no cousins yet"* —
and an unconsumed join layer looks exactly like a feature that works. Every
guard, baseline and normalisation around it stays green forever, because nothing
downstream can contradict them.

It also changes how the 82 remaining `bare-vs-tagged` splits should be priced.
Merging one is a per-lesson judgement with a real risk of asserting an etymology
that is not there, and today it buys nothing a reader can see. That is not an
argument for leaving them wrong — it is an argument for doing the consumer first
and the merges after, so each merge can be checked against a rendered page.

### What the options are

1. **Render it.** `book.ts` emits the authored `etymology` block; a generated
   cousins panel beside it would put the join in front of a reader. Needs a
   decision about what happens when the joiner and the authored prose disagree,
   which is the interesting question and not a small one.
2. **Widen `ROMANCE_COUSINS`.** Unrelated to consumption, but the default list
   is `french, italian, portuguese`, so a Portuguese lesson whose only cousin is
   Spanish still shows nothing — `PT-C24-queijo` and `FR-C26-dormir` both read
   `(none)` for this reason rather than for any slug fault.
3. **Say so in the module.** Cheapest, and worth doing whichever of the above
   happens: a header line recording that the layer is not yet consumed stops the
   next reader assuming its output is on a page somewhere.
