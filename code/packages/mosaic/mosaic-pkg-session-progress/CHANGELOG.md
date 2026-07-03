# Changelog

## Unreleased

- Added a light-theme stylesheet (`SessionProgress.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Added the `SessionProgress` Mosaic component package for review-session
  current, remaining, correct, and total counters.
- The component exposes label/value slots so hosts can bind shared Engram core
  session-progress JSON without target-specific layout forks.
