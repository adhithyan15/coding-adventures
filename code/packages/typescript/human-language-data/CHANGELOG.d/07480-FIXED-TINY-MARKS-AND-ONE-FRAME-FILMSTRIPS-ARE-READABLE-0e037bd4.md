### Fixed — tiny marks and one-frame filmstrips are readable

- `src/figure-filmstrip.ts`: a one-letter strip's heading and citation now
  wrap at no less than `MIN_TEXT_WIDTH`, the 310-unit width of a two-frame
  strip, and the figure is that wide. A ONE-frame strip (़, ्, ا, ه, ہ, ں,
  ゜, ー, 一 and seven Gujarati letters) used to wrap them at its single
  150-unit panel:
  three heading lines and up to ten citation lines. The panel keeps its size
  and its place at the left margin. Strips of two or more frames are already
  at least that wide and do not change.
- With script-ductus's new `penSizeFor`, the pen line and dot on the tiny
  marks ़, ं and ં are scaled to the mark, so the dot no longer covers it.
- Regenerated 32 filmstrip SVGs and 9 figure-hash shards (chinese, gujarati,
  hindi, japanese, marathi, marwadi, persian, sanskrit, urdu): 22 one-frame
  strips; the nukta strips HI-S140 and MW-W15 (both reasons); the anusvara
  strips GU-W03, HI-S148, MR-W02, MW-W03 and SA-S209; and the Gujarati
  sequence strips GU-R04, GU-R13 and GU-R19, which contain ં. No book
  chapter changed.
- Test: a one-frame strip is as wide as a two-frame strip, wraps its heading
  and citation the same way, and keeps its one panel at the margin. HL06 and
  the README describe both rules.
