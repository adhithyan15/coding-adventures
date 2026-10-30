# Changelog

All notable changes to `http1-fixed-request-body` are documented here.

## [Unreleased]

## [0.1.0] - 2026-10-29

### Added

- Added bounded non-copyable evidence binding one borrowed request body to its
  exact byte length.
- Added allocation-free shortest-decimal `Content-Length` value encoding.
- Added payload-free errors and body-redacting Debug output.

### Security

- Checks the complete decimal output capacity before any write.
- Explicitly excludes transfer coding and duplicate/conflicting framing policy.
