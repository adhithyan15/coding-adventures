# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- **Windows forms** of `open_owner_only_secret`,
  `read_owner_only_secret_from` and `check_owner_only_directory` (D18S
  P2.8b-1); they refused there before. A key file passes the policy
  `read_owner_only_secret` already enforces (the user owns it; a protected
  DACL with exactly the user's allow ACE), opened through held ancestors,
  sharing read only, never through a reparse point; the broker checks it
  again on the inherited handle and reads exactly the key from offset 0. A
  secret directory passes if the user, SYSTEM or Administrators own it and
  every allow ACE names one of those three (CREATOR OWNER too,
  inherit-only). The current user is read from the token pseudo-handle, an
  access check a confined broker could otherwise fail. Native tests on
  windows-2025 for each, and their refusals.
- `write_test_secret(path, bytes)`, behind a new `test-support` feature:
  the crate's own test writer (Unix mode 0600; Windows: the user as owner
  and a protected DACL with only the user's ACE), for other crates' tests
  that feed secrets to a loader built on this one. The CLI's vault tests
  needed it on windows-2025, where no portable `std` call sets that DACL.
  Production builds never enable the feature.
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
