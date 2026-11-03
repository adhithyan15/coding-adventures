# Changelog

## Unreleased

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.
- Added graph, node, and edge property bags with synchronized `weight` edge metadata.

## 0.1.0

- Add the DT00 undirected weighted graph package for Swift.
