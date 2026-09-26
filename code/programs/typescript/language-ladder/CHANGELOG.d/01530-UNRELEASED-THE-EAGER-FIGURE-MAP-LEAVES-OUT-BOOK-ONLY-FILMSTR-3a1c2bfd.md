## Unreleased — the eager figure map leaves out book-only filmstrips

`src/figures.ts` eagerly globs every committed book figure as a URL. HL-C443
added several hundred stroke-order filmstrips (`*-filmstrip.svg`), and the map
pushed the largest eager chunk to 507,112 bytes, over the 500 kB `check:bundle`
budget. The book places those filmstrips from derived targets and no lesson's
Markdown references one, so the app never asks for them. The glob now excludes
`*-filmstrip.svg`, and the chunk is back to 495,771 bytes.

Showing filmstrips in the app needs lazy loading and is tracked in HL-C443.
