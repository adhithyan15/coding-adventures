# Changelog

## 0.2.0 - Unreleased

- Migrate from Vitest and shared `../tsconfig.base.json` to standalone
  `tsconfig.json` / `tsconfig.test.json` with `node:test`,
  `@coding-adventures/test-bootstrap`, and `--test-coverage-lines=95`.
- Add VER15 CWE-20/190/400/1321 hardening (`MAX_SAMPLES`, finite/sparse/type
  validation, overflow-safe `heldValueAt`, safe-integer `sampleAtIndex`, and
  `Object.freeze`), plus `discrete-waveform.warrant`.
- Add `tests/hostile_inputs.test.ts` and `tests/pbt.test.ts` (SplitMix64 + `fuzzTarget`).

## 0.1.0

- Added sampled waveform representation with zero-order-hold reconstruction.
