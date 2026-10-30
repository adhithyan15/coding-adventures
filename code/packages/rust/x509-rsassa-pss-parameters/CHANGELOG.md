# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Bounded decoding of X.509 RSASSA-PSS signature parameters.
- Ordered explicit-field handling with RFC 4055 validator-default compatibility
  and DER salt DEFAULT-omission enforcement.
- RFC 4055 digest and MGF1 identifier decoding with redacted failures.
- Independent boundary coverage, standalone build fronts, and schema-v1
  zero-capability declaration.
