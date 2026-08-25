### Two constraints that moved the lessons

Two constraints bit here, and both are recorded because both changed the plan.

**Ordering.** `TA-C33-ezhutu` (sequence 970) already makes **ழ** and **த** its own
subjects inline — "**ழு** is **ழ** with the *u* sign" — so a strand lesson introducing
them afterwards would be claiming first contact it does not have. `TA-W16` has no
dependency on the other two, so it goes first, at sequence 965, immediately ahead of
that lesson. The cost is visible in R2 above and is worth naming rather than smoothing.

**The chapter-32 budget.** The strand's 3:1 cadence put two of these lessons inside
chapter 32. Chapter 32 was
already **at** the ramp policy's ceiling — six verb lessons, two atoms each, exactly the
`maxNewAtomsPerChapter: 12` budget — so interleaving there took it to **16** and broke
`chapterViolations`, the number `ramp.test.ts` calls the one that most directly measures
"do not throw many things at the reader at once."

That is the opposite of the point, so the cadence yields and chapter 32 is skipped
entirely. The three lessons sit at chapters 33, 34 and 35, which leaves one nine-lesson
gap after `TA-W13` and then resumes 3:1. Reading distance between consecutive strand
lessons stays at 4 or more everywhere, so the R1 rationale in `continuity.test.ts` still
holds.

Skipping chapter 32 was not only the gentler choice, it was the cheaper one. Measured
both ways: interleaving there would have cost a ramp violation (24 → 25), a payoff
representativeness failure (29 → 30, chapter 32 at 7/16), a fully-drivable chapter
(324 → 323) and three lessons of chapter-32 drivable prefix. Skipping it costs none of
those.

