# Criterion 4 counts distinct later LESSONS, so a zero-revisit atom needs TWO reviews — plan against slots, not atoms

The HL09 §3.1 reinforcement criterion is `revisits < 2`, and `measureContinuity`
computes `revisits` as **the number of distinct later lessons whose
`practises.knowledge` names the atom**. A single review lesson therefore contributes
**at most one** revisit to any given atom, no matter how many times it drills it.

The consequence is that the open-atom count is not the size of the job. Spanish's A1
residue read as **59 atoms** and was **72 slots**, because every atom sitting at
`revisits=0` needs two further passes rather than one. The gap is not evenly spread:
chapters 100–199 held 11 atoms in 18 slots with **seven at zero revisits**, so a
single review there would have left all seven open *while appearing to cover the
cluster* — the atoms named in the frontmatter, the lesson landed, the number barely
moving. That is the same failure mode as a wiring pass that accepts every prose hit:
coverage that looks complete because the surface count matches, while the underlying
requirement does not.

Plan against slots, verify against slots, and report both numbers so the discrepancy
cannot hide. `slots = Σ (2 − revisits)` over the atoms the gate calls thin.

Two related facts about the gate, both easy to get backwards:

- **It is a count, not a window check.** An atom can keep its `ReinforcementDefect`
  (it still misses R2/R3/R4) while the gate's verdict flips, because only `revisits`
  moved. Do not read "the defect is gone" as the success condition.
- **An atom in a stretch too short to contain even R1 produces no defect at all**, so
  it is invisible to criterion 4 rather than failing it. That is why the tail must be
  measured explicitly and why appending a lesson can *manufacture* a blocker.
