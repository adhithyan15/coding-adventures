# Changelog

## 0.2.0 - Unreleased

- Migrate from Vitest and shared `../tsconfig.base.json` to standalone
  `tsconfig.json` / `tsconfig.test.json` with `node:test`,
  `@coding-adventures/test-bootstrap`, and `--test-coverage-lines=95`.
- Add VER15 CWE-20/190/400/1321 hardening to `ConstantWaveform` and
  `SineWaveform` (finite checks, angular-frequency overflow guard,
  period/phase reduction, and `Object.freeze`), plus `analog-waveform.warrant`.
- Add `tests/hostile_inputs.test.ts` and `tests/pbt.test.ts` (SplitMix64 + `fuzzTarget`).

## 0.1.0

- Added analog waveform primitives with constant and sinusoidal waveforms.
