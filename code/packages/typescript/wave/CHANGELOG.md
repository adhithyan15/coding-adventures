# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Changed

- Migrate from Jest/ts-jest and shared `../tsconfig.base.json` to standalone
  `tsconfig.json` / `tsconfig.test.json` with `node:test`,
  `@coding-adventures/test-bootstrap`, and `--test-coverage-lines=95`.
- Add VER15 CWE-20/190/400/1321 hardening (`requireFiniteNumber`, `validateState`,
  `Object.freeze`, negative-zero normalization) and `wave.warrant`.
- Add `tests/hostile_inputs.test.ts` and `tests/pbt.test.ts` (SplitMix64 + `fuzzTarget`).
- Enforce finite parameters, angular-frequency overflow, and finite-time
  validation required by PHY01.
- Reduce time and phase before local trig evaluation and cover exact-zero,
  maximum-finite, and minimum-subnormal boundaries.
- Make both build entrypoints run the TypeScript compiler before node:test.

## [0.1.0] - 2026-03-22

### Added

- `Wave` class with constructor validation (amplitude >= 0, frequency > 0)
- `evaluate(t)` method computing `A · sin(2πft + φ)` using first-principles trig
- `period()` method returning `1/f`
- `angularFrequency()` method returning `2πf`
- Readonly `amplitude`, `frequency`, and `phase` properties
- Comprehensive test suite covering evaluation, periodicity, phase shifting, derived quantities, validation, and higher frequencies
- Literate programming style with inline explanations of wave physics
