# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Security

- Move the test toolchain from `jest` 29 to `jest` ^30.5.2, `@types/jest` to ^30.0.0, and `ts-jest` to ^29.4.14, which accepts jest 30. jest 29 reaches `braces` 3.0.3 through `micromatch`. `braces` has a high-severity stack-exhaustion advisory (GHSA-vfj7-8cjw-p6xm) and no patched release, so the only fix is the jest 30 tree, which no longer depends on it. A moderate `sprintf-js` advisory (GHSA-hp3w-g68c-fv3c) remains through `babel-plugin-istanbul` → `@istanbuljs/load-nyc-config` → `js-yaml` 3 → `argparse` 1. It has no upstream fix and is tracked in `code/specs/EXTERNAL-DEPENDENCY-ALERT-BACKLOG.md`.

### Changed

- Enforce finite parameters, angular-frequency overflow, and finite-time
  validation required by PHY01.
- Reduce time and phase before local trig evaluation and cover exact-zero,
  maximum-finite, and minimum-subnormal boundaries.
- Make both build entrypoints run the TypeScript compiler before Jest.

## [0.1.0] - 2026-03-22

### Added

- `Wave` class with constructor validation (amplitude >= 0, frequency > 0)
- `evaluate(t)` method computing `A · sin(2πft + φ)` using first-principles trig
- `period()` method returning `1/f`
- `angularFrequency()` method returning `2πf`
- Readonly `amplitude`, `frequency`, and `phase` properties
- Comprehensive test suite covering evaluation, periodicity, phase shifting, derived quantities, validation, and higher frequencies
- Literate programming style with inline explanations of wave physics
