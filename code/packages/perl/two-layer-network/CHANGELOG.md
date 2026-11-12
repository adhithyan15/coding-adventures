# Changelog

## [Unreleased]

### Added

- Added `BUILD_windows`, which runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries). The Windows build previously fell back to the POSIX-only `BUILD`.
