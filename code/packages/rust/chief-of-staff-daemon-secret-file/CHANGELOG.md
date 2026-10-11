# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- `check_owner_only_directory` (D18S P2.6d-3): a no-follow walk to a
  directory owned by the effective user with nothing granted to group or
  others. The launcher runs it on every directory holding secrets before
  each broker launch.
- `open_owner_only_secret`: the same no-symlink walk and owner-only checks as
  `read_owner_only_secret`, without reading (D18S P2.6d). The supervisor
  passes the descriptor to the one broker that needs the key.
- `read_owner_only_secret_from`: re-checks the policy on an open descriptor,
  then reads it with `pread` from offset 0, which neither depends on nor
  moves the offset it shares with the opener.

## [0.1.0] - 2026-08-09

### Added

- Add exact-length reads into zeroizing storage.
- Add descriptor-relative Unix no-follow traversal plus effective-owner and
  owner-only mode checks.
- Add Windows ancestor locking, reparse-point rejection, and protected
  current-user-only DACL validation.
- Add stable payload-blind failures and cross-platform regression coverage.
