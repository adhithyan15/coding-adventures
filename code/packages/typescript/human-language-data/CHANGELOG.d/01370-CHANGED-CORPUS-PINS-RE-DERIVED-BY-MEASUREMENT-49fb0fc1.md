### Changed - corpus pins re-derived by measurement

Every moved pin was re-derived as a set difference against `origin/main`, and
the direction of each mover is recorded at the assertion:

- `atomsTaught` 2650 -> 2652; `pre-A1` 877 -> 878; ramp-to-A1 1186 -> 1187;
  manifest `totalLessons` 1689 -> 1690 and `pen` 67 -> 68. The `pen` derivation
  comes from `writing-type` before the script block is considered, matching the
  `["writing-type","script-block"]` pair that 20 other Tamil lessons already
  carry, so the sight seam does not move.
- `atomsNeverRevisited` holds at 472, as does the 422-atom subset of it that
  also misses a window. Both are trades rather than washes, and they trade
  DIFFERENT atoms, which is worth separating because one set is a superset of
  the other. Both lose the same two: `TA-SCRIPT-READ-UUR-02` and
  `TA-SCRIPT-UU-VOWEL-01`, rescued from zero revisits by the re-reading above.
  The 472 set gains `TA-SCRIPT-UU-SIGN-01` and `TA-SCRIPT-READ-MUUNRU-02`, this
  lesson's own two atoms, never revisited because nothing follows them. The 422
  subset gains `TA-ETYMON-VIDAI-02` and `TA-LEX-VIDAI-01` instead — those two
  were already never-revisited at baseline and merely became window-measurable,
  the artifact described next, while TA-W19's own atoms miss the subset because
  at index 127 no window is evaluable for them at all.
- `missedByWindow.R2` 1809 -> 1808, and it goes DOWN. Three atoms leave —
  `TA-SCRIPT-READ-UUR-02` (revisits 0 -> 1), `TA-SCRIPT-UU-VOWEL-01` (0 -> 1)
  and `TA-SCRIPT-U-VOWEL-01` (1 -> 2).
- `missedByWindow.R4` 242 -> 243, with `TA-SCRIPT-THREE-NS-01` leaving
  (revisits 4 -> 5, missing R1/R2/R4 -> R1/R2).
- Every atom that ENTERS a window does so by one mechanism, and the arithmetic
  is exact in all four: the Tamil track was 127 lessons, and for each entrant
  `introducedAt + window.from = 127`, so that window's first position did not
  exist until this lesson made index 127 exist. R1 886, VIDAI pair at 126
  (126 + 1); R2, IVAR pair at 122 (122 + 5); R3 1307 -> 1309, UTAVU pair at 107
  (107 + 20); R4, PLEASE-REGISTER pair at 47 (47 + 80).
- For every one of those entrants the revisit COUNT is identical before and
  after, which is the check that separates an artifact from a regression: no
  existing reinforcement was broken by the insertion. TA-W19's own two atoms
  appear in no window at all, because at index 127 none is evaluable.

