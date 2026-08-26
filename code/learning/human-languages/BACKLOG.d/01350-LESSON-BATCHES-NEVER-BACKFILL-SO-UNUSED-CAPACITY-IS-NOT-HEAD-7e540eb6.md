## Lesson batches never backfill, so unused capacity is not headroom

Recorded here because the figure it corrects has already been printed in a merged
pull-request body as evidence, and evidence is what it is not.

Tranche 4 raised the bundler's `maxSize` grouping parameter from 49 kB to 56 kB,
took the emitted lesson-batch count from 401 down to 353, lowered the
request-count ceiling to match, and reasoned that the remaining 32% of unused
capacity was headroom the next few tranches could grow into -- "6.29 MB of fill
headroom before the batch count can grow again".

Measured on the next tranche:

```
before   353 batches   13,478,418 B total   32% of cap unused
after    359 batches   13,624,129 B total   32% of cap unused
```

Thirty-five lessons weighing **145,711 B** -- about 2.6 batches at the 56 kB cap,
and slightly *lighter* than tranche 4's thirty-five -- added **six** batches, and
the unused fraction did not move at all. A number that does not move when the
thing it supposedly measures does is not a measurement.

The slack is not one pool. Rolldown groups by track and then splits each track
greedily by size, so the tail batch of every *other* track is sealed and never
revisited; a Spanish tranche can only extend Spanish's tail. Corpus-wide slack is
stranded by construction, and the count therefore tracks corpus bytes roughly
linearly however much aggregate slack a report shows.

Fixed structurally rather than with a third bump: batches are now grouped by a
five-chapter range and the request budget is derived from the corpus band count
instead of hardcoded, so adding lessons inside a band moves neither side and
adding chapters moves both together.

**The generalisable check:** before treating unused capacity as headroom, ask
whether the allocator can *reach* it. Summing free space across N independently
sealed partitions answers a question nobody asked; the number that predicts
growth is the free space in the one partition the next write lands in. The same
error is available in disk allocators, shard maps and connection pools.

