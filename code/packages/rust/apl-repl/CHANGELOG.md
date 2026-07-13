# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- **`MAX_LINE_LEN` (64 KiB) caps a single physical line read from the input
  stream**, applied via the new `read_bounded_line` helper *before*
  `MAX_CONTINUATION_BUFFER`'s own check ever runs. `BufRead::read_line` has
  no length bound of its own — it grows until it sees `\n` or EOF — so a
  single, arbitrarily long physical line (no embedded newline at all) was
  previously fully buffered in memory regardless of the continuation-buffer
  cap. Found (LOW severity, given this crate's stdio-only threat model) by
  the security review of the `j-runtime`/`j-repl` PR, which mirrors this
  crate closely enough that the same gap applied here too. An oversized
  line is now rejected cleanly (`Error: line exceeds the 65536-byte limit;
  discarded`) with its remainder drained (one more bounded chunk) rather
  than left to be picked up mid-line by the next read.
- `read_bounded_line` reads raw bytes (`read_until(b'\n', ..)`) and only
  decodes UTF-8 after confirming the byte run ended at a genuine `\n`
  within the cap, rather than calling `BufRead::read_line` directly on the
  `Take`-wrapped reader — found by `/security-review` on this same change:
  `Take` truncates at an arbitrary byte offset with no notion of a
  character boundary, so a valid UTF-8 line only slightly over the cap
  could have a multi-byte character straddle that offset and make
  `read_line` report a spurious (and fatal — it aborted the whole session)
  `InvalidData` error instead of the intended clean "line too long"
  message.
- The "oversized?" check itself now requires **both** no trailing `\n`
  *and* the byte count actually reaching `MAX_LINE_LEN` — found by a
  second `/security-review` round on the fix above: `read_until` also
  stops with no trailing `\n` at genuine EOF (e.g. the very last line of
  input has no closing newline), and checking `\n`-absence alone
  misclassified that ordinary, short, valid line as "oversized" and
  silently discarded it. A real I/O error while draining an oversized
  line's remainder is now propagated rather than swallowed, for the same
  reason (a broken pipe there isn't the same thing as "no more bytes to
  discard").

## [0.1.0] - 2026-07-11

### Added

- Initial release — the interactive REPL and the **`apl` binary** (item
  MA-4e), a sibling of `matlab-repl`/`s-repl`/`r-repl` over the
  `apl-runtime` interpreter.
- `AplRepl` with a persistent workspace, `>> `/`... ` prompts, and `quit`/
  `exit`/EOF handling; `run()` drives a full stdio session.
- **Line continuation across an open `(`** — the only grouping construct in
  this language cut (no block keywords, no string type). Continuation lines
  are joined with a space rather than a real newline, since `apl.tokens`
  does not drop newlines inside `(...)` in this first cut.
- Errors are surfaced (`Error: …`) without ending the session.
- `MAX_CONTINUATION_BUFFER` (64 KiB) caps the pending-continuation buffer
  while a `(` is still unbalanced, discarding it with a clean error rather
  than growing without bound — found by `/security-review` (LOW severity
  under this crate's stdio-only threat model, fixed anyway for defense in
  depth) before this crate's first push.
