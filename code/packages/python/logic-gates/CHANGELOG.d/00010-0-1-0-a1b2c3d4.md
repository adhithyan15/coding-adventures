# Changelog

All notable changes to the logic-gates package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- Initial package scaffolding with pyproject.toml, src layout, and test structure
- `XOR_N(*bits)`: N-input XOR gate for parity computation; used by the Intel 8008
  gate-level simulator to compute the P (parity) flag via XOR reduction over 8 result bits.
  Returns 1 if an odd number of inputs are 1, 0 if an even number are 1.
  Unlike `AND_N`/`OR_N`, accepts 0 or 1 inputs (returns 0 for empty, identity for single).
