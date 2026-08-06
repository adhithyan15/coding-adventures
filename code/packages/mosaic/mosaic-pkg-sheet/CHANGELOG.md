# Changelog

All notable changes to `mosaic-pkg-sheet` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/) and
the package follows semantic versioning.

## 0.1.1 — 2026-08-06 — editing is real

### Fixed

- **`onNavigate`/`onFormulaChange`/`onEditCommit` regain their real payloads.**
  0.1.0 shipped these declared void, matching what `mosaic-emit-react` could
  actually dispatch at the time — a `Box`-rooted `Cell` (see
  `mosaic-pkg-grid`) couldn't carry a click payload through the emitter.
  [UI37](../../../specs/UI37-generic-payload-dispatch.md) fixed that at the
  emitter level; `mosaic-pkg-grid` 0.2.3 threads `row`/`col` through `Cell`
  to use it. `onNavigate(row, col)` and `onEditCommit(value: text)` now
  reach a consumer for the first time. Depends on `mosaic-pkg-grid` 0.2.3.

## 0.1.0 — 2026-08-06 — initial release, read-only

First release: filter box + sort-field `Select` + `mosaic-pkg-grid`'s
`Grid`, wired to task-core's `table(view)` projection. Shipped in
task-app's sheet view (v1 read-only — see task-app's own CHANGELOG for the
two bugs found and fixed building it: `mosaic-pkg-grid`'s `Cell.onClick`
never firing at all, and `mosaic-emit-react`'s `HostInput.onCommit` never
carrying a declared payload).
