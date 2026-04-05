# Changelog — wave (Swift)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-04

### Added
- Initial implementation of Swift wave package.
- `Wave` struct with amplitude, frequency, phase.
- Derived properties: `period`, `angularFrequency`.
- `evaluate(at:)` for computing displacement at a given time.
- Input validation (non-negative amplitude, positive frequency).
