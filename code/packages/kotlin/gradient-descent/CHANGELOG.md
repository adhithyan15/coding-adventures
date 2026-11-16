# Changelog — gradient-descent (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-11-15

### Added
- Initial implementation of Kotlin ML02 gradient-descent optimizer.
- `GradientDescent.sgd(DoubleArray?, DoubleArray?, Double)` with unfused IEEE-754 binary64 arithmetic and `@JvmStatic` companion object.
- Deterministic error precedence with closed `GradientDescentErrorCode` enum.
- CWE-400 pre-allocation vector ceiling of 1,048,576 elements.
- Full pre-pass non-finite input validation and numeric overflow refusal.
- 5-Class ML02 Optimizer Security Resilience Showcase test suite and JaCoCo coverage >=95%.
