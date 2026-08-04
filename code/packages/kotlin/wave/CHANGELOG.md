# Changelog — wave (Kotlin)

## Unreleased

### Changed
- Enforce finite construction, angular-frequency overflow, and finite-time
  validation required by PHY01.
- Use the local composite-build `trig` dependency with reduced time and phase.
- Cover zero-amplitude, maximum-finite, and minimum-subnormal boundaries.

## [0.1.0] — 2026-04-04

### Added
- Initial implementation: Wave data class with amplitude, frequency, phase.
- Derived properties: period, angularFrequency.
- evaluate(t) for displacement computation.
- Input validation via require().
