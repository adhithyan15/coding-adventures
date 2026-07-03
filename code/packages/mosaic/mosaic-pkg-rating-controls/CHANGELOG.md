# Changelog

## Unreleased

- Added a light-theme stylesheet (`RatingControls.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Added the `RatingControls` Mosaic component package for spaced-repetition
  answer grading controls.
- The component exposes label slots for localization or host-specific wording
  and emits the four review events expected by Engram's Rust core.
