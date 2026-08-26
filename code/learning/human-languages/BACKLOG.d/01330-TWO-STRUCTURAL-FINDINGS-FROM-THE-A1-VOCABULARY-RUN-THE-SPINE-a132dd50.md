## Two structural findings from the A1 vocabulary run: the spine has no room for a verb, and lesson batches never backfill

Tranche 5 (Spanish chapters 367-373) hit two walls that are not about content and
will hit every tranche after it. Both are recorded here because both changed what
was authored, and neither is visible from inside a single pull request.

### The A1 spine cannot host a verb, so the run to 600 is nouns and adjectives

`vocabularyOf` counts a headword toward a level when the lesson's curriculum
segment names a spine node whose `stage` is at or below that level. For Spanish
that leaves eleven usable nodes at or below A1, and in practice the vocabulary
tranches use three of them: `SPINE-DEFINITE-REFERENCE` ("mark out a specific
known thing"), `SPINE-ASK-LOCATION` ("ask where something is") and
`SPINE-COUNT-ONE-TO-FIVE` ("the cardinal numbers one through five").

None of those three can honestly host a verb. A `canDo` reading "I can say
*aprender*, *olvidar*, *necesitar*, *terminar* and *usar*, and mark out which
specific thing I mean" is not a capability statement; it is a slot being filled.

Ten verified verb candidates were dropped for this reason and this reason alone
-- `aprender`, `olvidar`, `necesitar`, `terminar`, `usar`, `lavar`, `romper`,
`bailar`, `tocar`, `oler`. Every one of them had cleared the headword screen, the
atom screen and the root screen, and every one had its etymology verified. They
were replaced with nouns and adjectives that fit the three nodes.

The pre-A1 nodes are the escape hatch and the precedent already exists: chapter 6
teaches `hablar`, `estudiar` and `trabajar` on `SPINE-POLITE-REQUEST-REPAIR`. But
that is a stretch too, and it was set two hundred chapters ago rather than chosen.

**What is actually missing** is an A1 node for the thing a learner does at A1 with
a verb -- naming an everyday action in the infinitive. Until one exists, the run
from 549 to the 600 floor is structurally restricted to things and their
properties, which is a curriculum shape nobody chose and which no gate reports.

### The "fill headroom" model of the lesson-batch count is wrong

Tranche 4 raised the bundler's `maxSize` grouping parameter from 49 kB to 56 kB,
took the emitted lesson-batch count from 401 down to 353, lowered the
request-count ceiling to match, and reasoned that the remaining 32% of unused
capacity was headroom the next few tranches could grow into.

It is not headroom, and the arithmetic that says it is has the wrong model of the
bundler. Measured on this tranche:

```
before   353 batches   13,478,418 B total   32% of cap unused
after    359 batches   13,624,129 B total   32% of cap unused
```

Thirty-five lessons weighing **145,711 B** -- about 2.6 batches at the 56 kB cap,
and slightly *lighter* than tranche 4's thirty-five -- added **six** batches. The
unused fraction did not move at all, because the slack is not one pool. Rolldown
groups by track and then splits each track greedily by size; the tail batch of
every *other* track is sealed and is never revisited, so a Spanish tranche can
only ever extend Spanish's tail. Corpus-wide slack is stranded by construction.

So the count tracks corpus bytes roughly linearly no matter how much aggregate
slack the report shows, and a `maxSize` bump buys one tranche of relief by
re-splitting everything at once. This is the third occurrence of the same
recurrence, and the comment in `vite.config.ts` already says a third should not
be answered with a fourth bump.

This is the argument for **#12918**: group batches by a **chapter range**, which
is a unit a reader actually navigates and a unit that does not grow when a track
gains lessons at its end. Recorded here rather than only in the issue because the
32% figure has now appeared in a merged pull-request body as evidence of
headroom, and it is not evidence of anything.

### A third, smaller one: an etymology can be its own evidence

`la alfombra` was going to be taught as Arabic for "the red one". That account was
printed in the Academy's dictionary for a long time. Corominas took it apart on a
ground worth naming: the redness had been read **out of** the proposed etymology
and then offered back as evidence **for** it, with a genuinely red-rooted
neighbour (`alhamar`, a coverlet) tangled into the record beside it.

Three of this tranche's other rewrites have the same shape -- a tidy, widely
printed proposal that the current specialists have quietly stopped endorsing
(`pecten`/`pecus`, `pulvis`/`pollen`, `gelu`/`glacies`). A hook that is beautiful
and old is exactly the hook to check, and checking it costs one lookup.

