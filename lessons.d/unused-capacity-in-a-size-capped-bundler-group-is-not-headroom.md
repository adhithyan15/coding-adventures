# Unused capacity in a size-capped bundler group is not headroom

Follow-up to "A grouping parameter is not a budget", and a correction to the reasoning
recorded there. Raising `maxSize` 49 kB → 56 kB took the lesson-batch count 401 → 353 and
left 32% of the aggregate cap unused. That 32% was written into a merged PR body as
"6.29 MB of fill headroom before the batch count can grow again". It is not headroom, and
the next tranche proved it:

```
origin/main   353 batches   13,478,418 B   32% of cap unused
+35 lessons   359 batches   13,624,129 B   32% of cap unused
```

Thirty-five lessons weighing 145,711 B — about **2.6** batches at the cap, and lighter than
the previous thirty-five — added **six** batches, and the unused fraction did not move at
all. Rolldown groups by track and *then* splits each track greedily by size, so every other
track's tail batch is sealed and never revisited. A Spanish tranche can only extend
Spanish's tail. **Aggregate slack in a partitioned bundler group is stranded by
construction.**

**Generalisable check:** before treating unused capacity as headroom, ask whether the
allocator can *reach* it. Summing free space across N independently-sealed partitions
answers a question nobody asked; the number that predicts growth is the free space in the
one partition the next write lands in. The same error is available in disk allocators,
shard maps and connection pools.

**How it was actually fixed, since "stop treating slack as headroom" is not a fix.** Group
by something the corpus *has* rather than by size — here a five-chapter band — so the count
follows a structural property instead of bytes. Then **derive the budget from that property
instead of hardcoding it**: count the bands and require the emitted chunks to correspond.
Adding lessons inside a band moves neither side; adding chapters moves both together; a
regression moves only one and fails. The ceiling stops needing to be raised, which is what
made it debt in the first place.

**And it exposed how weak the constant had been.** 353 was met on a corpus that only needed
281, so **72 batches of drift would have passed unremarked**. A constant sized once is a
gate that loosens every day the corpus grows without ever telling you.
