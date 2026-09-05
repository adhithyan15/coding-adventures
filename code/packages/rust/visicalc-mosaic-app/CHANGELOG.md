# Changelog

## Unreleased

- Accept positive measured viewportRows capacity, clamp to workbook bounds and
  preserve/reveal selection without weakening strict public resize validation.

## 0.1.0 — 2026-09-05

- Implement the standard Mosaic app and native ABI over spreadsheet-core.
- Own selection, edit buffering/commit/cancel, viewport resizing and snapshots.
- Replay the shared presentation contract and verify atomic error handling,
  snapshot restoration, runtime retry and native ABI lifecycle.
