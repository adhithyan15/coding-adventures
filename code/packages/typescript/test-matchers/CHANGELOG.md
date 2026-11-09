# Changelog

All notable changes to `@coding-adventures/test-matchers` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-08

### Added
- Extracted standalone zero-runtime-dependency test matchers and assertions package.
- Pure bit-exact deep equality engine (`valuesEqual`, `toValue`, `canonicalF64`, `encodeValue`).
- Complete `expect()` assertion suite:
  - Identity & equality: `toBe`, `toEqual`, `toStrictEqual`.
  - Type & presence: `toBeDefined`, `toBeUndefined`, `toBeNull`, `toBeNaN`, `toBeInstanceOf`.
  - Booleans & truthiness: `toBeTruthy`, `toBeFalsy`.
  - Numeric relations: `toBeGreaterThan`, `toBeGreaterThanOrEqual`, `toBeLessThan`, `toBeLessThanOrEqual`, `toBeCloseTo`.
  - Collections & strings: `toContain`, `toHaveLength`, `toMatch`, `toHaveProperty`.
  - Exceptional control flow: `toThrow`.
- Fully inverted `.not` chain on all matchers.
- Structured `AssertionError` with `actual` and `expected` properties.
