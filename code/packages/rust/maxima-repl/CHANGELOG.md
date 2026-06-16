# Changelog — coding-adventures-maxima-repl

All notable changes to this package are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
Semantic Versioning.

## [0.1.0] — 2026-06-16

### Added

- Initial release. The interactive `maxima` binary and the
  `coding_adventures_maxima_repl::run` driver.
- `MaximaRepl` over a persistent `MaximaSession`: `(%i«n») ` / `... ` prompts,
  line continuation until a `;`/`$` statement terminator (tracked outside `"`
  strings — with `\"` escapes — and outside `/* */` comments, and only at bracket
  depth 0, so a `;` inside a string, comment, or paren group does not terminate
  early, and a stray `"` in a comment does not wedge the prompt),
  `quit;`/`quit()`/`exit`/EOF handling, and non-fatal error reporting (a surface
  error prints and the session continues).
- The accumulation buffer is size-capped (at the runtime's `MAX_INPUT_LEN`): an
  input that never satisfies the continuation rule is submitted once it exceeds
  the cap (yielding a clean "too large" error) instead of buffering unbounded
  memory.

### Notes

- The symbolic sibling of `octave-repl`. The only Maxima-specific logic is the
  `;`/`$`-terminator continuation rule. See `code/specs/MA03-maxima-language.md`.
