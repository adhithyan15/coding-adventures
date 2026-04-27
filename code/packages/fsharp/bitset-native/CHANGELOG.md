# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-15

### Added

- Native F# wrapper over the Rust `bitset-c` library using .NET P/Invoke
- Disposable `Bitset` API that mirrors the pure F# bitset package while delegating core operations to Rust
- xUnit coverage for constructors, mutation, bulk operations, queries, equality, disposal, and conversion helpers
