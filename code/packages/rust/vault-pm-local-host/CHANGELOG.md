# Changelog

## 0.1.0

- Added platform-standard configuration, local-data, and cache path resolution.
- Added owner-only, no-link local root preparation on Unix and Windows.
- Added a persistent owner-only lock file with non-blocking cross-process
  single-writer exclusion.
- Added exact, bounded configuration loading plus owner-only atomic initial
  creation and compare-and-exchange through the live writer guard.
- Added closed, path-free diagnostics and platform security tests.
