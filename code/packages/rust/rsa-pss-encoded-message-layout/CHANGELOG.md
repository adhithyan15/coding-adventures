# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Checked, caller-bounded RFC 8017 EMSA-PSS `emBits` and `emLen` policy.
- Exact trailer, parameter-fit, and forbidden-high-bit validation.
- Borrowed non-copyable layout evidence retaining the complete encoded message,
  `maskedDB`, and `H` without allocating or granting signature authority.
- Stable redacted failures, independent boundary coverage, standalone BUILD
  fronts, and a schema-v1 zero-capability declaration.
