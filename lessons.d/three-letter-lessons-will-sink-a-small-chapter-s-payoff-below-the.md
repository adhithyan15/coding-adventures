---
category: Repo policy / workflow reminders
---

# Three letter lessons will sink a small chapter's payoff below the representativeness floor

Hindi chapter 96 needed three Devanagari vowels taught before it could be
written. Placement was chosen on one criterion — closure is measured in reading
order, so the letters had to sit **before the earliest word in the track that
uses them**. Chapter 20 already hosted two letter lessons, so the three new ones
went there.

`check:*` all passed. The snapshot diff did not: `payoff-surprise` went **13 →
14**, and the new finding was chapter 20 at `3/8 (0.38), below the 0.5 floor`.

The cause is arithmetic. A content chapter in this corpus is **three to five
atoms**, and its payoff lesson assesses two or three of them — a ratio around
0.6, with almost no slack. A letter run adds one atom per lesson and assesses
none of them in the chapter payoff, because the payoff is the chapter's *content*
lesson. Three letters is enough to halve the ratio of any small chapter:

```
ch20 before: 5 atoms, payoff assesses 3  ->  0.60  pass
ch20 after:  8 atoms, payoff assesses 3  ->  0.38  fail
```

The fix was placement, not a pin. Chapter 22 had **four atoms and a payoff
assessing all four** — a ratio of 1.00, and the only chapter in the band with
that much slack. Moving the three there gave 4/7 = **0.57**, above the floor,
returned chapter 20 to 0.60, and cost nothing else: chapter 22 is where the
track's first **ऊ** word lives, so the letters still land immediately before the
word that needs them.

Two things to do differently.

**Letter placement has two constraints, not one.** Position (before the earliest
word that needs the glyph) is necessary and not sufficient. Also check the host
chapter's **atom count and payoff coverage**, and prefer a chapter whose payoff
already assesses most of what it teaches. The numbers are cheap to get: count
`introduces` atoms per chapter and compare against the chapter's
`payoff.assesses` length.

**Read the snapshot diff, not only the gate results.** Every `check:*` gate
passed on the bad placement, and the full test suite would have too, because
`payoff-surprise` is report-only and its snapshot regenerates. The regression
was visible only in `git diff` over
`core/gentle-ramp-snapshots/<lang>.d/metrics/`. Diffing that directory after
regenerating is a ten-second check that catches report-only regressions before
they become the next chapter's baseline.
