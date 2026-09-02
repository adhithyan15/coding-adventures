### Added — a printed stroke-order filmstrip, and the ledger the book reads it from

- `penPathBetween(stroke, from, to)` draws the slice of a stroke between two
  fractions of its length, so a frame can ink the movement it is about instead
  of everything travelled so far.
- `DuctusStep` now carries `startFraction`, the point in the stroke where that
  labelled part begins. Read off the same arc-length measurement as `fraction`,
  so a frame boundary is the join rather than near it.
- `DuctusOptions.highlightSegment` (default `false`) mutes the part of the
  current stroke travelled before this frame and inks only the segment the
  caption names. The printed filmstrip turns it on; the live app keeps the
  existing whole-stroke shading, which several hundred per-glyph tests pin.
- `filmstrip-ledger.ts` writes `data/ductus/filmstrip-geometry.json` — the
  frames of the letters the book prints, as SVG fragments in one shared viewBox,
  with the citation and the font each was drawn from. Every number in it is read
  back out of the tree `ductusFilmstrip` produced, so there is no second
  geometry implementation to drift from the app's.
- `npm run generate:filmstrip-ledger` / `check:filmstrip-ledger`. The check also
  runs as part of `npm test`, so a stroke edited here and not regenerated fails
  this package rather than the book.

  Why the ledger exists rather than a direct import: this package cannot run under plain Node: `scriptdata.ts` reaches the canonical
Japanese/Perso-Arabic/Tamil/Urdu inventories through a Vite virtual module, and
the plugin serving it imports `human-language-data`. The book generator is a
Node CLI, and a direct import back from it would close a cycle the repository's
build tool rejects. The two therefore meet on generated data instead of on a
package boundary.
