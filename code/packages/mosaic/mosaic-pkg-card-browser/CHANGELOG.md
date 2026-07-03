# Changelog

## Unreleased

- Added a light-theme stylesheet (`CardBrowser.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).

- Added browser result metadata slots for card IDs, note IDs, template IDs,
  state labels, and selected-row values so host apps can target browser
  actions without parsing labels.
- Added the initial `CardBrowser` Mosaic component package for reusable Anki-style card browser/search surfaces.
