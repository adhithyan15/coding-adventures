### Added — a per-track ladder in the gap report, so the backlog is derived

The report already computed, for every track, what it touches, what has
structurally complete coverage, which level it is working on, and which
criterion blocks the next rung with a shortfall in that criterion's own units.
It printed **one summary line**:

```
levels with STRUCTURAL COVERAGE COMPLETE: 1 track at A1 (spanish);
  23 track(s) touch a level whose coverage is not complete
```

True, and useless for deciding what to do next. It names no track, no criterion
and no distance, so the only way to act on it was to write a throwaway script —
which is exactly what happened, and what this replaces.

```
per-track ladder (HL09 §3.1), nearest the next rung first:
  telugu      touches A2  complete none  working pre-A1  vocabulary 221/300 — vocabulary short 79
  tamil       touches A2  complete none  working pre-A1  vocabulary 208/300 — vocabulary short 92
  hindi       touches A2  complete none  working pre-A1  vocabulary 199/300 — vocabulary short 101
  ...
  persian     touches A2  complete none  working pre-A1  vocabulary  43/300 — vocabulary short 257
```

#### The ordering is the point, not decoration

Rows group by blocking criterion, then sort by shortfall **ascending within a
group**, so reading down a group is reading that part of the backlog in priority
order: the track nearest a rung it has not cleared is the cheapest real progress
available.

The grouping is not cosmetic. A first version sorted on the bare shortfall, and
the sentence written beside it — *"the row order is the priority order"* — was
false the moment two criteria appeared: shortfalls are in each criterion's own
units, so a track blocked by `spine-nodes short 1` would sort above one needing
79 headwords, while being the single track that authoring lessons **cannot**
advance at all. Every row is vocabulary-blocked today, so nothing in the corpus
or the tests would have caught it; security review did.

That is not hypothetical. The first run showed **telugu 79** and **tamil 92**
short of pre-A1 — the two tracks nearest the first structurally complete level
any non-pilot track would reach — while recent effort had gone to malayalam
(128 short) and hindi (101). A summary that hides which track is closest is how
that happens.

It also makes the corpus-wide shape impossible to miss: **every track except
Spanish has complete coverage of nothing**, and all 22 are blocked on the same
criterion at the same level.

#### Scoped, not total

Each row prints headwords **at or below the level in progress**, not the track's
total — the HL-C195 lesson. The total is context; the scoped number is the one
to author against. Spanish read `227/300` on the total while its blocker said
48, and sixteen new words on an A1 node moved the blocker by zero. A test pins
that the row carries the scoped figure and specifically *not* the total.

Report-only. No gate, no threshold, no behaviour change — the same numbers the
gate already computed, printed where they can be read.
