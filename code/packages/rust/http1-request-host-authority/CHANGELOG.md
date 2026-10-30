# Changelog

All notable changes to `http1-request-host-authority` are documented here.

## [Unreleased]

## [0.1.0] - 2026-10-29

### Added

- Added allocation-free validation for bounded ASCII DNS hostnames.
- Added optional non-zero port validation and shortest-decimal encoding.
- Added lowercase authority encoding into caller-owned output.
- Added payload-free errors and hostname-redacting Debug output.

### Security

- Rejects URI delimiters, whitespace, controls, Unicode, IP literals, legacy
  numeric address spellings, empty labels, oversized labels, and labels with
  leading or trailing hyphens.
- Checks complete output capacity before any write.
