# A grouping parameter is not a budget

`language-ladder` fails at >353 lazy lesson batches, and separately caps the largest batch.
Content growth walks into the count ceiling every few tranches.

The reflex is to read both numbers as budgets and refuse to touch either. But `maxSize` in
`vite.config.ts` is a **bundler grouping parameter**: raising it 49 kB → 56 kB took the
measured count **401 → 353**, moving the real ceiling *down* by 48 while the corpus grew by
35 lessons. Raising the *count* would have been the violation; this is its opposite.

Two things make that legitimate rather than convenient, and both should be checked before
reaching for it: the change had **in-repo precedent** (a previous author did 32 kB → 49 kB
for the identical recurrence and wrote it into the file), and the size increase stayed far
inside the budget that actually protects the browser (54,688 B against a 500 kB eager-chunk
limit — about 11%).

**And a ceiling that may fall must actually fall.** Lower the pin to the new measurement in
the same commit, or the slack you just created silently becomes room for the next
regression to hide in.
