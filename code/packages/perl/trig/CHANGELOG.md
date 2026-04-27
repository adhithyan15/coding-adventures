# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.1.0] - 2026-03-22

### Added

- Initial implementation of trigonometric functions from first principles
- `sin_taylor` — sine via Maclaurin series (20 terms)
- `cos_taylor` — cosine via Maclaurin series (20 terms)
- `radians` — degree to radian conversion
- `degrees` — radian to degree conversion
- `PI` constant to full double-precision accuracy
- Range reduction to [-pi, pi] for accuracy with large inputs
- Comprehensive test suite (special values, symmetry, Pythagorean identity, large inputs, conversions)
