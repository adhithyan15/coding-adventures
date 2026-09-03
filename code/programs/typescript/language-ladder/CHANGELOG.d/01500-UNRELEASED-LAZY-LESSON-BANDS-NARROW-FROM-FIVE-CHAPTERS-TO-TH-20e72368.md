## Unreleased — lazy lesson bands narrow from five chapters to three

- `LESSON_BAND_CHAPTERS` **5 → 3**. German's chapter-13 migration added 27
  lessons whose ids all begin `GE-C09-`; ids band by their **own** number rather
  than by the book chapter the lesson was assigned to, so all 27 landed in one
  band and pushed German 5–9 past the 256 kB backstop in `vite.config.ts`. The
  bundler split it, that was the second split in the corpus, and
  `check:bundle` failed with the remedy in its own message.
- **Width 4 was measured and rejected.** It clears today — 451 batches, zero
  splits, largest batch 211 kB — but German has four hand-written chapters left,
  all of them sized as splits, arriving with `GE-C10-` through `GE-C13-` ids. At
  the measured ~3.1 kB per lesson those project band C12 back onto the backstop
  by the last of them, which is a gate that fails again in two PRs' time.
- **Width 3 has room for all four.** 589 batches, zero splits, largest batch
  191 kB. The two German bands the remaining migrations feed project to about
  176 kB and 171 kB with all four chapters landed, each leaving a third of the
  backstop unused. 589 lazy batches over 4,100+ lessons is ~7 lessons a batch,
  nowhere near the one-request-per-lesson fan-out this grouping replaced.
- `BAND_SPLIT_SLACK` **1 → 0** in `scripts/check-bundle.mjs`. The debt was one
  Spanish band the backstop had to carve; at width 3 nothing splits at all, so
  the number is a real zero that fails on the first regression rather than
  absorbing one.
