# Changelog

All notable changes to the TaskApp Web Components parity host are documented
here. This host is a CI validation surface and is not a published release
artifact.

## [Unreleased]

### Added

- Added light and dark package-expanded Custom Element entry points.
- Added a real `task-wasm` host bridge with the shared TaskApp controller and
  browser persistence contract.
- Added an acceptance gate that drives create, complete, restore, and delete
  through the emitted controls and keeps startup failures visible.
