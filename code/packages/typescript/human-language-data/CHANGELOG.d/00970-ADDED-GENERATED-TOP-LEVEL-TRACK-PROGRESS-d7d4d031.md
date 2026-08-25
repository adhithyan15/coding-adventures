### Added — generated top-level track progress

- Derive every Human Languages index row from `core/languages.json`, canonical
  lessons, realization maps, authored book chapters, and the generated-book hash
  manifest instead of repeating hand-maintained chapter and lesson claims.
- Add `generate:progress` and the byte-for-byte `check:progress` publication gate;
  a new registered language appears even when it has no lessons or book yet.

