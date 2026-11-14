# Changelog

## Unreleased

### Fixed

- SEC-011 (CWE-400/770): `Framer.Feed` buffered input without limit when no LF arrived. The unterminated tail is now capped at 511 bytes (510 content plus a CR; RFC 1459 and RFC 2812 section 2.3: 512 including CRLF). Past the cap the partial line is dropped and input is skipped up to the next LF (fail closed, same silent discard as a complete overlong line; no signature changes, so no version bump). `Reset()` also clears the discard state. New public constant `IrcFraming.MAX_PENDING_BYTES`. Ported from the Rust fix (PR #19359).

## 0.1.0

- Add F# IRC framer with partial buffering, CRLF/LF extraction, reset, and RFC 1459 length enforcement.
