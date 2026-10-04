# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Fixed

- Await React's documented focus-restoration effect in the document lifecycle
  regression so slower macOS CI hosts do not race the assertion against it.

## [0.1.0] - 2026-10-03

### Added

- Added the controlled React editor for site settings, document lifecycle,
  persistent undo/redo, and all eight default Content IR block forms.
- Added immutable insert, replace, reorder, and remove helpers that compose the
  authoring core's complete-body command.
- Added keyboard-complete controls, deterministic focus restoration, busy
  state, polite status, bounded failure alerts, and visible focus styling.
- Added exact-key declarative plugin slots and a frozen least-authority bridge
  request rather than accepting same-realm plugin JSX or callbacks.
- Added pre-allocation list/table limits, synchronous action exclusion, private
  deep-frozen plugin snapshots, bidi-safe attributed labels, bounded plugin
  deadlines, unmount cancellation, and nearest-survivor focus restoration.
- Added browser, hostile-input, persistence-failure, plugin-boundary, and
  structural-edit coverage above the FM09 thresholds.
