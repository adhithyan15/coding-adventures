---
category: Repo policy / workflow reminders
---

# A hand-inserted spine ordinal passes in core/spine.d but fails in each track's curriculum.d/spine

**The two directories follow different rules, and only one of them says so.**
`src/shard-cli.ts` documents the friendly one at length:

> The ordinals are spaced by ten — `0010`, `0020`, … — so a node can be inserted
> between two others as `0015` without renaming its neighbours. `--check` treats
> that prefix only as an ordering coordinate.

That is true of `core/spine.d/`, which is the directory the comment is about —
`0135-SPINE-READ-SIGNS-AND-NOTICES.json` has sat between `0130` and `0140` since
it was minted. Reading it as a general rule is the mistake.

**Each track's `<track>/curriculum.d/spine/` is checked by a different gate.**
`tests/curriculum-shards.test.ts` rebuilds the ledger and re-shards it, then
compares the FILENAMES it would have written against the ones on disk. The
writer always emits the dense stride, so a hand-inserted `0235-` fails there
even though the same trick is fine one directory up. Inserting
`SPINE-READ-PRACTICAL-TEXTS` at position 24 of 41 therefore cost **414 renames**
— 18 files in each of 23 tracks — and the first full run failed 23 times, once
per track, with a diff that looked alarming and meant only "renumber".

**What to do.** When adding a spine node anywhere but the end, derive the
canonical name for every shard from the core node order and rename in
descending destination order so no rename lands on a file still to be moved:

```python
order = [json.load(open(f))["id"] for f in sorted(glob.glob("core/spine.d/0*.json"))]
want  = f"{(order.index(node_id) + 1) * 10:04d}-{node_id}.json"
```

Renaming ascending overwrites; renaming descending does not. And run
`npx vitest run tests/curriculum-shards.test.ts` BEFORE the full suite — it
takes two seconds and answers the question that five minutes of full run also
answers.
