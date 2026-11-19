# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-19

### Added

- Exact selected-suite equality before repository-owned Finished verification.
- Internal immutable transcript snapshotting with non-copyable evidence
  retaining the transcript, snapshot, and Finished verdict.
- Consuming verification that owns the selected transcript on success and
  withholds it on every redacted failure.
- SHA-256 and SHA-384 success coverage plus stable redacted mismatch failures.
