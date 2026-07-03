# Changelog

## Unreleased

- Added a light-theme stylesheet (`DeckStatsPanel.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Added the `DeckStatsPanel` Mosaic component package for deck-scoped total,
  new, due, learning, and hidden review counters.
- The component exposes label/value slots so hosts can bind shared Engram core
  deck-stat JSON without target-specific layout forks.
