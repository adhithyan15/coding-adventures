### Fixed — the ramp corpus snapshot loads the corpus once, at import

- `tests/ramp.test.ts`'s six whole-corpus cases (two snapshot, four script-ramp) each called
  `loadEverything()` and `measureRamp()` inside the test body, about 10s each
  on an idle machine. In a loaded full-suite run the first case crossed its
  30s budget while every assertion held. The corpus is now loaded and measured
  once at import (no per-test budget), the pattern `book-cli` and
  `chapter-modality-book` already use; all six read that one report and
  now take about a millisecond. No timeout raised, no assertion changed.
