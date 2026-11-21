# Changelog

## 0.2.0

- Migrated to standalone `tsconfig.json` + `tsconfig.test.json` (removed `../tsconfig.base.json` dependency).
- Wired `@coding-adventures/test-bootstrap` (`SplitMix64`) into `tests/pbt.test.ts` and `tests/hostile.test.ts`, and updated test script coverage excludes (`--test-coverage-exclude=../** --test-coverage-exclude=dist-test/tests/** --test-coverage-lines=95`).
- Updated `BUILD`, `BUILD_windows`, `.gitignore` (`coverage/`), and `package-lock.json` to mirror `oscillator` and build `@coding-adventures/test-bootstrap` first.

## 0.1.0

- Added the reusable TypeScript PCM encoding stage with:
  - `PCMFormat` metadata validation
  - `PCMBuffer` container with timing helpers
  - `floatToPcm16`, `encodeSampleBuffer`, and `samplesToPcmBuffer`
