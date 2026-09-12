# Changelog

## Unreleased

### Fixed — the sheet now exposes native table semantics on Compose (#14843)

VisiCalc's grid is the only `HostTable` in the product and Compose did not
recognise it as a table: no `collectionInfo`, no `collectionItemInfo`. Every
accessibility gate passed regardless, because each cell existed and was
correctly named — a screen reader simply had no way to know it was a table.

The cause was in the emitter, not here: its shape predicate required exactly
one child per row, and `RowHeaderGrid` opens each row with a fixed corner or
row-header cell before the `For`.

Degradations **8 → 7**; the render script's pin moves with them. The remaining
seven are `table-focus`, `table-wheel-shift` and `authored-table-cell`, all
gated on `backend != React` with no per-backend predicate.


### Added — VisiCalc can be rendered on Compose (#14829)

`conformance/compose/VisiCalcScreenshots.kt` and `scripts/render-compose.sh`
render the sheet to a PNG in one command. The third product to get a render
harness, after Trestle (#14799) and Engram (#14828), and the one where looking
pays off most directly: a grid is the hardest thing here to judge from a
semantics tree, because "every cell exists and is named" is true of a grid
whose headers sit nowhere near their columns — which is exactly what #14829
was.

Measured on the first reproducible render, against the semantics tree:

- **#14829's primary defect is fixed.** Column headers now lay out at an 80px
  pitch, matching the data columns' 80px pitch (right edges 113, 193, 273,
  353, 433). The issue's original measurement had five headers spanning 45px
  against the same five data columns spanning 338px — a different scale
  entirely. #14830's `Box` wrap and UI60 (#14828) between them fixed it.
- **The corner cell is still `0 x 24` at origin `(0, 0)`** — the remaining
  half of #14829. The colgroup's leading `Col (width: 48)` is not inside the
  `For`, and only the `For` is scanned for widths, so the row-header column
  has no threading path.
- **Columns Q-Z collapse to zero width** (#14842). Ten columns present in the
  semantics tree, correctly named, occupying no space; every accessibility
  gate passes on them.
- **VisiCalc is not native-complete on Compose** (#14843) — 8 degradations,
  all accessibility and table semantics. The script pins that count rather
  than asserting `nativeComplete`, so a ninth cannot appear unremarked.

Not a pixel-diff gate; a strict baseline needs a pinned font stack and
renderer, which is a separate decision (#14798).


## 2026-09-08

- Add a themed empty-workbook introduction above the existing editable grid.
  Rust supplies emptiness; the shared component owns the guidance and styling.

- Export a themed Mosaic startup component with loading/error content and Retry,
  preserving VisiCalc as the default project root.

- Add shared Open/Save controls and a compact file-status toolbar in both themes.
  The standard Rust adapter owns snapshot validation and file-operation outcomes.

## Unreleased

- Add a compact, themed selection readout with overflow truncation and complete
  Rust-owned cell descriptions for the host's atomic polite live region.

- Move through the workbook with measured wheel/trackpad row shifts while keeping
  realization bounded. React physical extent now spans the whole workbook; native acceptance remains.

- Use RowHeaderGrid with pinned absolute row labels, semantic headings and
  aligned 80px data columns. Keep selection clear of the leading header.

- Fit the sheet to the remaining app height and forward measured React table
  capacity to the shared Rust adapter while preserving the selected cell.

- Exercise authored scroll overflow alongside the primitive default after the
  shared React style-merging fix.

- Bound the sheet in HostScroll and pin the column headings while it scrolls.
  Match native browser control colors to the authored light/dark theme.

- Establish shared warm-paper and forest themes, workbook typography, formula
  chrome, sheet label, keyboard hints and distinct selected/editing cell states.
- Wrap narrow toolbar content and contain the wide sheet within its own frame.

- Size the root to at least the viewport height using Mosaic's corrected `vh` unit.

## 0.1.0

- Compose the root VisiCalc application with shared Mosaic controls and themes.
- Connect the web consumer to the standard Rust application lifecycle.
