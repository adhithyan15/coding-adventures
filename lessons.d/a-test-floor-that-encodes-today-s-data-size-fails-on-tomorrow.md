---
category: Testing & coverage
---

# A test floor that encodes today's data size fails on tomorrow's

2026-09-15.

A per-plan test compared the entry order read from a document against the order
read from its shard files, then guarded against both being empty:

```ts
expect(fromShards).toEqual(fromMonolith);
expect(fromShards.length).toBeGreaterThan(100);
```

The `100` was fine for four migrations, because the first documents sharded all
had hundreds of entries. Then a smaller one joined and CI said:

    AssertionError: expected 31 to be greater than 100

A **true statement about a perfectly healthy changelog**. Nothing was wrong
with the document, the shards, or the ordering. The number was a fact about the
data that existed the day the test was written, frozen into an assertion.

The tell is worth naming: the failure message is arithmetically correct and
tells you nothing about a defect. That is what a stale constant looks like from
the inside, and the tempting fix — lower it until green — produces a number with
even less meaning than the first one.

The fix is to find the number that has a REASON. The guard existed to stop
`toEqual` passing on two empty arrays, and separately the project already had a
threshold for "small enough that a shared file is not the bottleneck": about
twenty entries. So the floor became `MIN_SHARDABLE_ENTRIES = 20`, named and
documented, and it now fails for a document that should not have been
registered at all rather than for one that is merely smaller than the first
four.

**How to apply.** When a literal threshold appears in a test, ask what it is a
fact ABOUT. If the answer is "the size of the data when I wrote this", it will
fail on legitimate data later and teach whoever hits it to edit the number. If
the answer is a rule the project already holds, name the constant after the rule
and cite it.

Related: a number that never moves is not a measurement; an unasserted
measurement is not a gate.
