# Changelog

## 0.2.0

- Migrated test runner from Vitest / `--experimental-strip-types` to standalone `tsconfig.json` + `tsconfig.test.json` + `node:test` + `@coding-adventures/test-bootstrap` with `--experimental-test-coverage --test-coverage-lines=95`.
- Updated `BUILD`, `BUILD_windows`, `.gitignore`, and `package-lock.json` to mirror `oscillator` and build `@coding-adventures/test-bootstrap` first.
- Hardened against CWE-1333 (ReDoS): replaced regex parsing with a bounded linear-time ASCII scanner.
- Hardened against CWE-1321 (Prototype Pollution): switched from plain object mapping to `Map<string, number>` and explicitly rejected prototype properties (`__proto__`, `constructor`, `prototype`).
- Hardened against CWE-400 (Resource Consumption): capped note input string length to `MAX_NOTE_STRING_LENGTH = 32` and bounded octave ranges `[-100, 100]`.
- Hardened against CWE-20 / CWE-190 (Improper Input Validation / Integer Overflow): enforced `Number.isSafeInteger` for octaves/semitones and `Number.isFinite` for frequencies.
- Added local `note-frequency.warrant` synced with `code/specs/note-frequency.warrant`, plus `tests/hostile.test.ts` and `tests/pbt.test.ts` (SplitMix64).

## 0.1.0

- Added the initial note-to-frequency package.
- Added note parsing for natural notes plus single sharps and flats.
- Added equal-tempered frequency mapping anchored at `A4 = 440 Hz`.
- Added tests that lock down the shared parity vectors across language ports.
