# Changelog

## Unreleased

- Added a light-theme stylesheet (`DeckOptionsPanel.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).

- Added the initial `DeckOptionsPanel` Mosaic component for shared Anki-style
  deck scheduler option controls, including learning/relearning step lists and
  daily-limit/interval/multiplier fields.
- Added native checkbox controls for Anki-style bury-new-siblings,
  bury-review-siblings, and bury-interday-learning-siblings deck options.
