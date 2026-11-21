# Changelog

## [0.2.0] - 2026-11-20

### Changed

- Migrated to standalone `tsconfig.json` + `tsconfig.test.json` (removed `../tsconfig.base.json` dependency).
- Wired `@coding-adventures/test-bootstrap` (`SplitMix64`) into `tests/pbt.test.ts` and `tests/hostile.test.ts`, and updated test script coverage excludes (`--test-coverage-exclude=../** --test-coverage-exclude=dist-test/tests/** --test-coverage-lines=95`).
- Updated `BUILD`, `BUILD_windows`, `.gitignore` (`coverage/`), and `package-lock.json` to mirror `oscillator` and build `@coding-adventures/test-bootstrap` first.

## [0.1.0] - 2026-04-22

### Added

- Browser Web Audio sink for mono signed 16-bit PCM buffers.
- PCM validation, PCM-to-float conversion, `AudioBuffer` creation, and
  promise-based playback scheduling.
- Fake-`AudioContext` tests so CI can validate the browser sink without a real
  audio device.
