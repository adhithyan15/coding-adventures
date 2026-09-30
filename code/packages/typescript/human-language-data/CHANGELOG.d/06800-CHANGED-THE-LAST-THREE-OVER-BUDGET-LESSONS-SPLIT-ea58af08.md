### Changed — the last three over-budget lessons split

`PA-C07-hona`, `PA-C07-khana` and `RU-C03-govorit` introduced four atoms each, and were the last lessons in the corpus above the three-atom budget. Each keeps its first three atoms and hands the rest to a continuation placed straight after it. The pins move accordingly:

- **Punjabi:**
  - the lesson budget goes from 818 to 820, and the session map to 820 rows with a new migration hash;
  - every pinned position after chapter 7 moves by 2;
  - the reinforcement counts are re-measured: R2 −1 and R3 −2 are closed windows, and no window is lost.
- **Russian:** the lesson budget goes from 695 to 696.
- `tests/ramp.test.ts`: the steepest-lesson snapshot now asserts that the corpus has no lesson over the atom budget. A new over-budget lesson fails the test by name.
