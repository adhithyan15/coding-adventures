# Changelog

## Unreleased

- Added a light-theme stylesheet (`ReviewActions.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Added the `ReviewActions` Mosaic component package for Anki-style undo, bury,
  suspend, and mark controls.
- The component exposes label slots for localization or host-specific wording
  and emits action events expected by Engram's Rust event bridge.
