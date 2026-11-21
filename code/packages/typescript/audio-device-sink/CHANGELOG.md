# Changelog

## 0.2.0

- Migrated test runner from Vitest / `--experimental-strip-types` to standalone `tsconfig.json` + `tsconfig.test.json` + `node:test` + `@coding-adventures/test-bootstrap` with `--experimental-test-coverage --test-coverage-lines=95`.
- Updated `BUILD`, `BUILD_windows`, `.gitignore`, and `package-lock.json` to mirror `oscillator` and build `@coding-adventures/test-bootstrap` first.
- Hardened against CWE-1321 (Prototype Pollution): closed set validation for error kinds and rejection of prototype property keys.
- Hardened against CWE-400 (Resource Consumption): added pre-flight `Array`/`Int16Array` and streaming length caps plus bounded error/backendName strings.
- Hardened against CWE-20 / CWE-190 (Improper Input Validation / Integer Overflow): enforced `Number.isSafeInteger` and capacity bounds across `PcmFormat`, `PcmPlaybackBuffer`, `validateFormat`, and `frameCount`.
- Added local `audio-device-sink.warrant` synced with `code/specs/audio-device-sink.warrant`, plus `tests/hostile.test.ts` and `tests/pbt.test.ts` (SplitMix64).

## 0.1.0

- Add backend-neutral PCM playback contract and no-op audio sink.
