# Changelog

## Unreleased

### Fixed
- Express the shuffle rejection threshold without unary minus on an unsigned
  value so MSVC `/WX` accepts the sampler; preserve its seeded schedule.
- Enforce the complete optional metadata group and validate every metadata
  string before replay or serialization, preserving empty invocation values.

## 0.1.0

- Add reusable paired collection, exact analysis replay, and the indexed-array driver.
- Add strict signed-64-bit JSON validation, fixture conformance, generated tests, and installable CMake targets.
- Reject signed overflow from caller-supplied arrays in both public batch helpers; validate extrema, overflow, and cancellation.
- Accept any nonblank workload producer identifier during single-report replay while keeping the array CLI workload unchanged.
