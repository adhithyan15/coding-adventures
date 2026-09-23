---
category: Repo policy / workflow reminders
---

# A track's review-lesson TYPE is a per-track property; check what the track already uses before copying another track's

A corpus-wide programme was adding retrieval-only lessons to one language track after another. Six
tracks in, the shape was settled: `type: review`, no `introduces.knowledge`, atoms listed under
`practises.knowledge`. The seventh track was German, and the same shape would have been wrong.

German has **no `review` lesson anywhere**. Its 347 lessons break down as:

```
201 word   36 phrase   36 grammar   35 practice   25 etymology
  9 writing   3 reading   1 sound   1 practice-mix
```

`practice` is German's word for the thing every other track calls `review`. Both sit outside
`CONTENT_TYPES` — the set that gates whether a lesson contributes a headword — so either satisfies
the reinforcement criterion identically and neither moves the vocabulary blocker. **Nothing would
have failed.** Validation passes, the gate clears, the tests go green.

What it would have done is introduce the first `review` lesson into a track that had never had one,
inside a tranche whose entire claim is that it changes nothing about the track except retrieval. A
silent inconsistency that a later reader would have to explain — and the commit message would have
said the opposite.

**The check is one command, run before authoring:**

```
grep -h "^type:" <track>/lessons/*.md | sort | uniq -c | sort -rn
```

If the type you were about to use has a count of zero, the track has a different word for it. Look
at what it does use for retrieval-only material and match that.

**The general shape of this error is assuming a corpus-wide convention from a sample of tracks that
happened to agree.** Two other constraints in the same programme went the other way and are worth
recording beside it, because the difference is the point:

- `spine_node` must equal its path segment's `spine_node` — measured at **353 of 353** in one track
  and **347 of 347** in the next. Two independent confirmations, zero exceptions. That is a hard
  constraint, and can be relied on.
- Path rank *happening* to agree with book sequence is **not** such a constraint. It held throughout
  German and failed in Sanskrit, where the script lessons sat at graph rank 40 while sitting near the
  end of the book. Checking it per track is cheap; assuming it is how a tranche gets placed wrong.

The distinction is whether you measured it on this track or inherited it from the last one. A
convention you verified is a fact about the corpus; a convention you carried across is a guess with
a track record.
