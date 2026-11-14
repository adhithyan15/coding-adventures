# Changelog

All notable changes to `http1-fixed-request-body` are documented here.

## [Unreleased]

### Added

- Added non-copyable evidence binding one exact validated `Content-Length`
  field to the canonical shortest-decimal length of one exact fixed body.

### Security

- Rejects non-`Content-Length` names and noncanonical or mismatched values with
  payload-free errors while retaining no rejected field or body bytes.
- Keeps duplicate-field, transfer-coding, surrounding field-set, credential,
  request authorization, transport, and external-effect policy explicitly out
  of scope.

## [0.1.0] - 2026-10-29

### Added

- Added bounded non-copyable evidence binding one borrowed request body to its
  exact byte length.
- Added allocation-free shortest-decimal `Content-Length` value encoding.
- Added payload-free errors and body-redacting Debug output.

### Security

- Checks the complete decimal output capacity before any write.
- Explicitly excludes transfer coding and duplicate/conflicting framing policy.
