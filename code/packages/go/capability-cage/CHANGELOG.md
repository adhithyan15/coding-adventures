# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- D21's three nondeterminism categories (VER00 §6.3): `CategoryRand`,
  `CategoryThread`, `CategoryUI`, and actions `ActionSpawn`,
  `ActionMessage`, `ActionEvent`. The closed taxonomy grows from 19 to 25
  pairs: `rand:read`, `thread:spawn`, `thread:message`, `ui:read`,
  `ui:write`, `ui:event`.
- `ReadRandom(m, p)` checks `rand:read:*` and `Spawn(m, role, body)` checks
  `thread:spawn:{role}`. Both delegate to two new `Backend` methods,
  `ReadRandom` (`crypto/rand` in `OpenBackend`) and `Spawn` (a goroutine),
  so a test backend can serve seeded entropy and schedule workers itself.
  `Spawn` refuses a nil body.

### Changed

- **Breaking for custom backends**: `Backend` gains `ReadRandom` and `Spawn`.
  No implementation outside this package exists in the repository.

### Security

- `NewManifest` now rejects category/action combinations outside Spec 13's
  closed 19-pair taxonomy before a manifest can be observed through `Has`,
  `Check`, or `Capabilities`.
- Added exhaustive conformance against the shared language-neutral taxonomy
  fixture.

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
