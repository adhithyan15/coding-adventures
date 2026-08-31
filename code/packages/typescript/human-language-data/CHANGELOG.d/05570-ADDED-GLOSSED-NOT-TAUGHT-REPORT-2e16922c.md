### Added

- `npm run report -- --glossed-not-taught <track>` now emits a reproducible
  native-script review queue: every token found in that track minus every token
  taught in a lesson headword, with occurrence counts and lesson IDs. Use
  `--format json` for machine-readable output. The result is intentionally a
  report rather than a gate because etymological mentions and English-side
  descriptions still require human classification.
