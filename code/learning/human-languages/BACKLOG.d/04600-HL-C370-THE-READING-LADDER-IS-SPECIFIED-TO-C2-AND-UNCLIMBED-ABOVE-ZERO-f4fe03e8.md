## HL-C370 — the reading ladder is specified to C2 and unclimbed above zero

`reading-reach.test.ts` now measures what HL-C369 said nothing measured: each
track's longest connected passage against the stimulus length its own task shape
declares. The first run is the finding.

**Twenty-seven task-shape inventories. Twenty-three publish a stimulus length.
Exactly one track has a passage at all.**

Hindi is the striking case: it declares a complete reading ladder at all seven
levels, and every rung reads zero.

| level | shortest part | longest part | reached |
|---|---:|---:|---|
| pre-A1 | 1 | 1 | 0/3 |
| A1 | 70 | 90 | 0/3 |
| A2 | 150 | 220 | 0/3 |
| B1 | 220 | 340 | 0/4 |
| B2 | 400 | 530 | 0/4 |
| C1 | 650 | 750 | 0/4 |
| C2 | 950 | 1050 | 0/4 |

Spanish reaches 2 of 4 published parts at A1 — the two 20–30 word notice parts —
with a longest passage of 61 words against parts needing 150–175 and 175–210.

**The cheapest work in the corpus is here.** Thirteen pre-A1 inventories declare
a stimulus minimum of **one word** — chinese, gujarati, hindi, japanese, latin,
marathi, marwadi, persian, punjabi, russian, urdu — and all read zero purely
because those tracks contain no `comprehension` block at all. A pre-A1 reading
part is satisfied by a one-word passage. Eleven tracks are one authoring session
away from a rung they currently score nothing on.

Remember reading arrives in threes: a skill atom introduced once and never
revisited revokes the track's level on the reinforcement criterion, which is how
Spanish briefly fell from A1 to pre-A1.

Four inventories — arabic/A1, french/A1, french/pre-A1, german/A1 — are excluded
from the comparison and say why: DELF, Goethe and ALPT do not publish a stimulus
word count, and the shapes record that under `notPublished`. They are reported as
excluded rather than dropped, because a silent drop is how a measurement quietly
stops covering four of its twenty-seven cases.

Housekeeping noticed while filing this: **two backlog entries share rank
`04590`** — HL-C368 and HL-C369, from branches that were open at the same time.
`check:doc-shards` passes, so ordering between them is merely ambiguous rather
than broken, but the rank is positional and a third collision would be worth
preventing.
