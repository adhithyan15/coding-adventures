### Fixed — 語's five-component stroke is not "short"

- `src/strokes/japanese.ts`: the 語 ductus captioned 五's second stroke
  "draw a short stroke down and left". That stroke runs from the top bar to
  the base (KanjiVG 04e94), as the standalone 五 ductus already says. It now
  reads "draw the stroke down and left", the same caption 五 uses; the path is
  unchanged. Only the stroke-ownership non-Tamil data hash moves.
