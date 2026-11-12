# Changelog — irc-proto

## [Unreleased]

### Fixed

- `required_capabilities.json` rewritten to the canonical schema-v1 shape (version, package, capabilities objects, justification) so the hardened capability analyzer accepts it.

## [0.1.0] — 2026-04-12

### Added
- `Message` struct with prefix, command, params fields
- `ParseError` type implementing `std::error::Error`
- `parse(line: &str) -> Result<Message, ParseError>`
- `serialize(msg: &Message) -> Vec<u8>`
- Comprehensive unit tests
