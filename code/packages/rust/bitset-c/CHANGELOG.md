# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-15

### Added

- Stable C ABI wrapper around the Rust `bitset` crate using opaque handles
- Error reporting helpers for invalid binary strings, null handles, invalid UTF-8, size overflow, and panic containment
- Unit tests that exercise handle lifecycle, mutation, bulk operations, integer conversion, and error propagation
