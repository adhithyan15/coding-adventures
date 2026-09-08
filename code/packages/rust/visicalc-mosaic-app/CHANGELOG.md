# Changelog

## Unreleased

- Derive the empty-workbook introduction from committed content across all sheets,
  retaining it during pending/cancelled edits and recomputing it after restore.

- Add protocol-2 workbook file requests and transactional Open completion. Preserve
  edits on failure/cancellation and require applying edits before Save. Report
  unsupported legacy hosts explicitly; keep pending file IDs out of snapshots.

- Describe selected values, blank cells, formula results/errors, commits and
  cancellations through standard polite announcements. Keep typing and viewport
  changes quiet and expose the same committed-cell summary to generated hosts.

- Clamp relative viewport shifts without retargeting selection or pending edits.

- Derive absolute row labels alongside the rendered window without adding a
  workbook data column or changing selection/edit coordinates.

- Accept positive measured viewportRows capacity, clamp to workbook bounds and
  preserve/reveal selection without weakening strict public resize validation.

## 0.1.0 — 2026-09-05

- Implement the standard Mosaic app and native ABI over spreadsheet-core.
- Own selection, edit buffering/commit/cancel, viewport resizing and snapshots.
- Replay the shared presentation contract and verify atomic error handling,
  snapshot restoration, runtime retry and native ABI lifecycle.
