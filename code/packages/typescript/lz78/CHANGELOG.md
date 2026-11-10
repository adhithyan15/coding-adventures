# Changelog

## [Unreleased]

### Security

- Hardening (CWE-789): deserialiseTokens clamps the untrusted token count to the data present; decode stops once originalLength is reached (quadratic expansion before truncation) and on a dangling dictIndex (previously a TypeError), and no longer uses push(...seq), which overflows the stack on long sequences.
- Added tests/hostile_headers.test.ts (wired into the npm test scripts): hostile headers checked for bounded time, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial LZ78 implementation (CMP01 spec). 37 tests.
- `encode`, `decode`, `compress`, `decompress`, `serialiseTokens`, `deserialiseTokens`.
- ESM, TypeScript strict mode, Vitest test suite.
