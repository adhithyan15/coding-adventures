# Changelog

All notable changes to `form-urlencoded` are documented here.

## [Unreleased]

## [0.1.0] - 2026-10-29

### Added

- Added bounded allocation-free form serialization for borrowed UTF-8 pairs.
- Added exact `+` space handling and uppercase percent encoding.
- Added order, duplicate-name, and empty-field preservation.
- Added payload-free failures and input-redacting Debug output.

### Security

- Checks every component and the complete encoded output before any write.
- Explicitly leaves secret-output wiping and audited custody to callers.
