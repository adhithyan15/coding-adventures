---
category: Repo policy / workflow reminders
---

# A tranche's sequence blocks must be spaced by each run's real lesson count, not a guessed stride

**What went wrong (Persian A1 tranche).** The tranche was split into runs of
ten and nine chapters. Each run's `SEQ_START` was written by hand. The first
four runs were spaced 530 apart, which is enough for a ten-chapter run: 50
word lessons plus 2 reviews, each 10 apart. The fifth run started only 480
after the fourth, as if the fourth held nine chapters. Four lessons shared
sequences with the next run. Three suites caught it: `continuity`
(duplicate-sequence), integration ("keeps chapter numbers non-decreasing")
and the spine validator (prerequisite with a later sequence).

**Fix.** The fifth and sixth runs start at 6020 and 6500. The tranche was
regenerated from a clean tree.

**Do differently.** Derive each run's `SEQ_START` from the previous run's
real size: `start + 10 * (5 * chapters + 2) + 10`, never by eye. After
generating, check that no sequence repeats before running anything else. A
one-line Counter over the lessons' `sequence:` fields is enough.
