# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `irc-framing` crate: a stateful
  byte-stream-to-line-frame converter for the IRC protocol.
- `irc_framer_new` / `irc_framer_free`, `irc_framer_feed` (append raw bytes,
  overflow-guarded growth), `irc_framer_frames` (drain complete CRLF/LF-stripped
  lines into an `IrcFrames` batch), `irc_frames_free`, `irc_framer_reset`, and
  `irc_framer_buffer_size`.
- Frames are raw byte slices (`{data, len}`) — a frame may hold any byte value.
  Overlong lines (content > 510 bytes, RFC 1459 §2.3) are silently discarded.
- The frame scan uses a cursor and drains the consumed prefix in a single move
  (the Rust original drains after each line); the result is identical, and an
  allocation failure mid-scan leaves the buffer intact for a retry.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): CRLF and lone-LF
  framing, partial buffering across feeds, the CR/LF split boundary, the empty
  frame, the 510-byte overlong-line rule, reset, and multi-feed sequences.
