---
category: Repo policy / workflow reminders
---

# curriculum-membership order fields are 0-indexed and dense, so appending a lesson takes the count of existing members, not the count plus one

Fifteen new arabic `curriculum-membership.d/<id>.json` shards were generated from a
plan that recorded each host segment's position **1-indexed**, because that is how
the segment dump printed it. Every one was off by one, and `loadEverything()` threw
before a single test ran:

    Error: curriculum path 'AR-PATH-014': lesson 'AR-R10-the-name-exchange-cold'
           has order 3, expected 2

`exactDenseOrder` in `src/curriculum-membership.ts` requires the orders within a
segment — and within each extension — to be exactly `0, 1, 2, …` with no gaps. So
for a segment that already holds *n* lessons, the appended lesson's `pathOrder` is
**n**, not n+1. The same holds for each entry in `extensions[].order`.

The failure is loud and immediate, which is the good news: it is a loader throw,
not a silent misplacement. The cost is that it throws inside `loadEverything()`,
so *every* measurement script and *every* test fails at once with one message
about one lesson, and the real shape of the problem — fifteen shards, all wrong —
is invisible until you fix the first and watch the second appear.

**What to do differently.** When a script derives an order from a listing, print
the derived value **beside the value an existing shard already carries** for the
same segment and eyeball the two. One `cat` of any committed shard in that segment
would have caught this:

    $ cat arabic/curriculum-membership.d/AR-R41-calling-and-asking-again.json
    { "id": "...", "pathSegment": "AR-PATH-038", "pathOrder": 2, ... }

That lesson is the **third** member of `AR-PATH-038`. The shard says `2`. There is
no ambiguity to reason about once you have looked.
