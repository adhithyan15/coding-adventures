### Fixed — four books explained their own build system to the reader

- Four `payoff.summary` fields ended with a note addressed to the gap report:
  *"Chapter 17 has no terminal practice lesson, so the payoff is the last lesson by
  sequence (4 of 12 atoms, below the 0.5 floor)."* Printing that under the chapter
  title broke the very rule that got the old blurb removed. Moved to a
  non-printed `payoff.note`; a test rejects it returning.

