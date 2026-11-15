# Changelog

## 0.2.0

- Migrated test runner from Vitest to `@coding-adventures/test-bootstrap` (TF01).
- Removed Vitest dependency and removed `vitest.config.ts`.
- Hardened against CWE-1321 (Prototype Pollution): closed set validation for error kinds and rejection of prototype property keys.
- Hardened against CWE-400 (Resource Consumption): added streaming length cap preventing memory exhaustion on unbounded iterables.
- Hardened against CWE-20 (Improper Input Validation): added runtime type assertions and strict `Number.isFinite`/`Number.isInteger` guards.
- Added property-based testing (PBT, VER05a) and sample normalization fuzz suites.
- Added Warrant specification `code/specs/audio-device-sink.warrant`.

## 0.1.0

- Add backend-neutral PCM playback contract and no-op audio sink.
