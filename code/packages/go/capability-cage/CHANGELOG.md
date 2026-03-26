# Changelog

All notable changes to this package will be documented in this file.

## [1.0.0] - 2026-03-25

### Added

- Initial implementation of `capability-cage` library
- `Manifest` type: immutable compile-time capability declaration constructed via `NewManifest`
- `EmptyManifest`: pre-built zero-capability manifest for pure-computation packages
- `Capability` struct: declares a single OS-level permission with `Category`, `Action`, `Target`, `Justification`
- Category constants: `CategoryFS`, `CategoryNet`, `CategoryProc`, `CategoryEnv`, `CategoryFFI`, `CategoryTime`, `CategoryStdin`, `CategoryStdout`
- Action constants: `ActionRead`, `ActionWrite`, `ActionCreate`, `ActionDelete`, `ActionList`, `ActionConnect`, `ActionListen`, `ActionDNS`, `ActionExec`, `ActionFork`, `ActionSignal`, `ActionCall`, `ActionLoad`, `ActionSleep`
- `CapabilityViolationError`: error type returned when a manifest check fails, with actionable remediation hint
- `Manifest.Check`: returns `CapabilityViolationError` if operation not declared
- `Manifest.Has`: boolean check without error allocation
- `Manifest.Capabilities`: returns a copy of the capability list for introspection
- `Backend` interface: abstracts I/O delegation (OpenBackend for stdlib, CageBackend for D18)
- `OpenBackend`: default backend delegating to Go stdlib (`os`, `net`, `exec`, etc.)
- `WithBackend`: swaps the package-level default backend; returns restore function for use with `defer`
- Secure filesystem wrappers: `ReadFile`, `WriteFile`, `CreateFile`, `DeleteFile`, `ListDir`
- Secure network wrappers: `Connect`, `Listen`, `DNSLookup`
- Secure process wrappers: `Exec`, `Signal`
- Secure environment wrappers: `ReadEnv`, `WriteEnv`
- Secure time wrappers: `Now`, `Sleep`
- Secure stdio wrappers: `ReadStdin`, `WriteStdout`
- Glob matching: bare `*` (any target), `*.ext` (same directory), literal (exact match)
- Path normalization via `path.Clean` to prevent traversal attacks
- Comprehensive test suite: 80+ tests covering all wrappers, glob cases, error messages, integration
