---
category: Repo policy / workflow reminders
---

# A new chapter is registered in four places and three of the failures surface only as shard-identity errors

Adding chapter 81 to a language track meant writing three lesson files and their curriculum-membership
records. Validation passed. The full suite then failed **four separate shard tests**, none of which
named a missing chapter:

```
kannada/chapters.json chapters identity set differs: missing [81], unexpected []
book-generation combined identity set differs: missing [kannada/0081], unexpected []
book-generation targets identity set differs: missing [], unexpected [kannada/0081]
```

A chapter is registered in **four** places, and only the first two are authored:

| # | where | authored or generated |
|---|---|---|
| 1 | `<track>/chapters.d/00NN.json` — the chapter record and its payoff | **authored** |
| 2 | `core/book-generation.d/targets.d/<track>-00NN.json` — the `.tex` output target | **authored** |
| 3 | the generated book-hash manifest | generated |
| 4 | the generated narration-hash manifest | generated |

3 and 4 come from `generate:books` and `generate:narration`, and **they only emit a chapter that 1 and
2 already declare**. So the order is **declare, then generate, then check**. Running the generators
first — the habit from every ordinary tranche, where no new chapter is involved — produces nothing for
the new chapter and silently leaves the manifests short, which is why the first failure reads like a
shard-tooling bug rather than a missing file.

The middle error above is the tell, and it is worth reading carefully. Adding the target *alone* flips
`missing [kannada/0081]` into `unexpected [kannada/0081]`: `targets` is checked against the generated
book manifest and `combined` against the generated narration manifest, so declaring one side without
regenerating the other just moves which set disagrees. Two errors that look contradictory are one
cause.

**Two smaller constraints on the chapter record itself:**

- `label` is charset-checked against `/^[A-Za-z0-9:_-]+$/`. A transliterated title carrying a
  diacritic is rejected at load — `ch:ka-ulidavu`, not `ch:ka-uḷidavu`. The error is clear when you
  reach it, but it arrives one full generate cycle after the file was written.
- The record wants a `payoff` naming a lesson, its kind, and the atoms it assesses. For a chapter that
  introduces nothing, the payoff is the retrieval itself — say so in the `note` rather than leaving a
  reader to wonder why a chapter declares no atoms.

**The operational form:** when a tranche adds a chapter rather than only lessons, write both authored
registrations *before* the first generate, and treat `check:shards` as the gate that tells you whether
you got all four — the lesson-level validator will not, because from its point of view nothing is
wrong.

Related: [[a-track-s-review-lesson-type-is-a-per-track-property-check]] — also a per-track structural
fact that nothing enforces globally, and also cheap to check once you know to look.
