# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- `CompletionPort::get` now reports a dequeued packet before it considers the wait
  timed out. Previously a failed operation that completed with Win32 error 258
  (`WAIT_TIMEOUT`) was returned as `TimedOut` and its packet was lost, leaving the
  caller's operation outstanding forever. Found by the security review of the
  `tcp-reactor` Windows engine.
- Document the opaque-pointer Windows contracts and keep their safe wrappers
  clean under the Rust 1.97 Clippy raw-pointer lint.

## [0.1.0] - 2026-04-17

### Added

- `CompletionPacket` and `CompletionPort`
- Windows wrappers for completion-port creation, association, posting, and dequeue
- non-Windows unsupported fallback
