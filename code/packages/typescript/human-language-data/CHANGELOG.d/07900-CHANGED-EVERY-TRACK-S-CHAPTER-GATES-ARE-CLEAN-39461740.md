### Changed — every track's chapter gates are clean

The last thirteen `chapter-payoff-not-representative` findings are fixed in
the curriculum (Kannada 10, 13, 16; Malayalam 13; Tamil 10, 13, 16, 19, 27;
Hindi 8, 14; Spanish 210, 266). The report's `chapterPayoffsNotRepresentative`
goes from 13 to 0, and `chapterGateCleanTracks` from 18 to 23.

- `tests/chapters.test.ts`: the zero-debt pin adds hindi, kannada, malayalam,
  spanish and tamil, so it now lists all 23 tracks.
- `tests/curriculum-digests/`: the hindi, kannada, malayalam and tamil pins
  are rewritten for the eleven new checkpoint lessons and their consolidation
  extensions. Spanish's graph does not change.

No source in `src/` changed.
