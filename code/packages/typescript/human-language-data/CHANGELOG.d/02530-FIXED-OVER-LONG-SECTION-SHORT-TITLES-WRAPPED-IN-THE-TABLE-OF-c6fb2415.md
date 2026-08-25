### Fixed - over-long section short titles wrapped in the table of contents (HL-C109d)

- Cut `sectionShortTitle` to a budget of 40 display columns, the corpus's 99th
  percentile, so a month or weekday list no longer wraps a one-line TOC entry.
- Count combining marks as zero columns and East Asian wide forms as two; cut at
  a word boundary, and keep a single over-wide word intact rather than mid-word.
- Drop a trailing separator with the item it joined, so a cut list does not read
  as `sal - se - ...` with something missing from the middle.
- 17 section lines in 17 chapters change. All 22 books now build with 0 overfull,
  0 underfull and 0 missing characters.

