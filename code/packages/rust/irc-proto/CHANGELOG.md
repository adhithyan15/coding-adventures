# Changelog — irc-proto

## [0.1.0] — 2026-04-12

### Added
- `Message` struct with prefix, command, params fields
- `ParseError` type implementing `std::error::Error`
- `parse(line: &str) -> Result<Message, ParseError>`
- `serialize(msg: &Message) -> Vec<u8>`
- Comprehensive unit tests
