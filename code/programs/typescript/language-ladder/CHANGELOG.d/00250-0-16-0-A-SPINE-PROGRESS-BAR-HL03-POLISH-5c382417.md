## 0.16.0 — a spine progress bar (HL03 polish)

- **A slim progress bar under the "Concept N of 186" line**, showing how far
  along the whole spine the walk has reached — a sense of the journey's scale
  that the bare count doesn't convey at a glance (a thin sliver at concept 1, a
  half-full bar at ~93, full at the end).
- New pure `spineProgress(cursor, length)` in `sequence.ts` returns the fraction
  reached in `[0, 1]`, counting the current concept (cursor 0 of 10 → 0.1),
  clamping an out-of-range cursor and returning 0 for an empty spine. 3 new tests
  (228 total) with a control that bites: a naive `cursor/length` would read 0 at
  the start, so the test pins 0.1. Width is set via `style.width` (no innerHTML).
  Verified in a real browser (seeded to concept 93 → the bar sits at ~50%).

