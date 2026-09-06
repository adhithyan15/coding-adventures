# Changelog

## Unreleased

- Move through the workbook with measured wheel/trackpad row shifts while keeping
  realization bounded. A whole-workbook scrollbar remains unfinished.

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
