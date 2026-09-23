## HL-C424-76d7dbdd — Clearing Telugu pre-A1 is three blockers, not 79 words, and adding the words makes one of them worse

**Status: OPEN.** Found by reading the per-track ladder the gap report now prints
(`#15904`), then reading the track's full blocker list rather than only its
worst one.

Telugu is the **nearest track in the corpus to a rung it has not cleared**, and
that makes it the cheapest real progress available — which is why it is worth
costing properly before authoring anything.

### What the ladder says, and what it omits

The ladder row prints the worst blocker only:

```
telugu   touches A2  complete none  working pre-A1  vocabulary 221/300 — vocabulary short 79
```

Read alone, that says "author 79 headwords". The full blocker list says
otherwise:

| criterion | short | detail |
|---|---:|---|
| vocabulary | 79 | 221 distinct headwords at or below pre-A1, against 300 |
| reinforcement | 50 | 50 atoms at or below pre-A1 revisited fewer than twice (46 etymology hooks waived) |
| atom-budget | 1 | `TE-C08-dayachesi` teaches 4 atoms against a budget of 3 |

All three must clear. §3.1 is a conjunction.

### The trap: the obvious fix makes the second blocker worse

Every new content lesson introduces at least one atom, and criterion 4 requires
**every** atom at or below the level to be revisited at least twice. So 79 new
headwords do not merely leave `reinforcement` at 50 — they add up to 79 more
atoms that each need two revisits, taking the reinforcement debt to roughly
**130 atoms / 260 revisit slots**.

A tranche of 79 vocabulary lessons would move `vocabulary` to 0 and
`reinforcement` from 50 to ~129. The track would be *further* from pre-A1 than
it is now, by the gate's own arithmetic, while the headline number looked
finished. That is the same shape as HL-C421: a real number, read as the whole
answer.

### What is actually needed

1. **~79 new pre-A1 headwords**, each introducing as few new atoms as it can.
2. **Two revisits for every new atom.** `practice` and `review` lessons are the
   instrument: `constants.ts` excludes them from `CONTENT_TYPES`, so they
   satisfy reinforcement **without** inflating the headword count or the atom
   budget. This is the load-bearing fact for planning the tranche.
3. **Two revisits for the 50 existing under-revisited atoms**, which are mostly
   `TE-LEX-C01-*` and `TE-GRAMMAR-C01-*` — chapter 1 material introduced once
   and never returned to.
4. **Fix `TE-C08-dayachesi`**, 4 atoms against a budget of 3.

### Adjacent, and deliberately not folded in

`ramp.chapters` reports **five** Telugu chapter-level budget violations, worst
`chapter 2 at 27 atoms against a budget of 12` across 18 lessons. Those are not
counted by §3.1's criterion 3, which measures per-lesson, so they do not block
pre-A1 — but a chapter carrying more than twice its atom budget is a pacing
defect on its own terms, and chapter 2 is where a Telugu reader starts.

### Why this is worth doing first anyway

Telugu (79) and Tamil (92) are the two tracks nearest the first structurally
complete level **any** non-pilot track would reach. Of the 23 tracks, only
Spanish has cleared a rung, and only A1. The other 22 are all blocked at
pre-A1, all on vocabulary as their worst criterion — so whatever shape this
programme takes for Telugu is the template for twenty-one more, exactly as
Spanish was meant to be.

Tamil's blockers have the same three criteria in the same order
(92 / 73 / 1), so the estimate transfers.

**Do not open this as a 79-word tranche.** The reinforcement arithmetic above is
the reason, and it is measured rather than argued.
