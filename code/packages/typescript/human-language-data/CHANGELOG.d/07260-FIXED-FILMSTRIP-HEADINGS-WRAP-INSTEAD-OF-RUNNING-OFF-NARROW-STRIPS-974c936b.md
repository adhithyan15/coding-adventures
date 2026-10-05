### Fixed — filmstrip headings wrap instead of running off narrow strips

- `src/figure-filmstrip.ts` printed the "How it is written — …" heading as
  one `<text>` line whatever the strip's width. A strip of one or two frames
  is only 150 or 310 units wide, so a summary such as "one unbroken stroke ·
  2 movements" ran past the figure's right edge and was clipped mid-word on
  the printed page (Japanese し and へ, Gujarati હ, Persian and Urdu single
  letters, Chinese 人 and 二, among others).
- The heading now wraps with the same `wrapFigureText` estimate the citation
  footer uses. Each extra line moves the frames down by one heading line
  (15 × 1.25 = 18.75 units). A " · " separator stays on the word before it, so
  no wrapped line begins with "·".
- A heading that fits on one line lays out exactly as before. Only figures
  whose heading was too wide change: their committed SVGs and the generated
  figure hashes are regenerated with `generate:figures`.
- `tests/figure-filmstrip.test.ts` pins both cases: a two-frame strip with a
  long summary wraps to two lines and moves its frames to y = 60.75, and a
  short heading stays on one line with the frames at y = 42. The fixture's own
  heading ("2 strokes · 1 pen lift · 2 movements" over two frames) was one of
  the clipped ones, so the first layout test's expected y moves 42 → 60.75.
