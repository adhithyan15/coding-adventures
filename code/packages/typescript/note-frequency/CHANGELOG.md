# Changelog

## 0.2.0

- Migrated test runner from Vitest to `@coding-adventures/test-bootstrap` (TF01).
- Hardened against CWE-1321 (Prototype Pollution): switched from plain object mapping to `Map<string, number>` and explicitly rejected prototype properties (`__proto__`, `constructor`, `prototype`).
- Hardened against CWE-400 (Resource Consumption): capped note input string length to 32 characters and bounded octave ranges.
- Hardened against CWE-20 (Improper Input Validation): added runtime type assertions and `Number.isFinite`/`Number.isInteger` checks for all parameters and outputs.
- Added property-based testing (PBT, VER05a) and hostile input fuzzing suites.
- Added Warrant specification `code/specs/note-frequency.warrant`.

## 0.1.0

- Added the initial note-to-frequency package.
- Added note parsing for natural notes plus single sharps and flats.
- Added equal-tempered frequency mapping anchored at `A4 = 440 Hz`.
- Added tests that lock down the shared parity vectors across language ports.
