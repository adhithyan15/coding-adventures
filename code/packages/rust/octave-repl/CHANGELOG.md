# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-06-16

### Added

- Initial release — the interactive REPL and the **`octave` binary** (item
  MA-3e), a sibling of `matlab-repl` over the `octave-runtime` interpreter.
- `OctaveRepl` with a persistent workspace, `octave> `/`... ` prompts, and
  `quit`/`exit`/EOF handling; `run()` drives a full stdio session.
- Line continuation across open brackets and unterminated blocks, counting the
  Octave `endX` terminators (and `until`) as block closers at bracket depth 0;
  `#` and `%` line comments skipped.
