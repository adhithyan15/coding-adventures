# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-03

### Added

- Added create-or-load persistence that never overwrites an existing credential.
- Added canonical bounded reads and zeroizing returned secret storage.
- Added descriptor-relative Unix no-follow traversal, owner/mode validation, and
  mode-`000` to mode-`0600` publication after durable writes.
- Added Windows ancestor locking, reparse-point rejection, and explicit protected
  current-user-only DACL creation and validation.
- Added creator-race, invalid-content, unsafe-object, permission, and stable-error
  coverage.
