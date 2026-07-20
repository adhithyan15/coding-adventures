# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- **`JRepl::feed` now bounds `self.buffer` *before* growing it, not
  after** — the previous order always ran `self.buffer.push_str(line)` (or
  `push(' ')` + `push_str(line)` for a continuation) first and only then
  checked `self.buffer.len() > MAX_CONTINUATION_BUFFER`, so a single,
  caller-supplied `line` that was itself already oversized was fully
  copied into `self.buffer` (an O(n) copy, possibly reallocating) before
  the very check meant to bound it ever ran. Found while propagating
  `maple-repl::MapleRepl::feed`'s identical `/security-review` fix (MP-4,
  #8647) to this crate's sibling REPLs — this crate's own shipped
  `read_bounded_line` never feeds `feed` a `line` longer than
  `MAX_LINE_LEN`, but `feed` is a `pub fn` on a `pub struct`, so any other
  embedder calling it directly with an attacker-supplied, unbounded `&str`
  needed the same bound at this layer too, not just at the one shipped
  call site. The buffer is now never grown past the cap; an oversized
  `line` is rejected immediately (buffer cleared, same
  `"...continuation limit; discarded"` message as before).
- **`MAX_LINE_LEN` (64 KiB) caps a single physical line read from the input
  stream**, applied via the new `read_bounded_line` helper *before*
  `MAX_CONTINUATION_BUFFER`'s own check ever runs. `BufRead::read_line` has
  no length bound of its own — it grows until it sees `\n` or EOF — so a
  single, arbitrarily long physical line (no embedded newline at all) was
  previously fully buffered in memory regardless of the continuation-buffer
  cap. Found (LOW severity, given this crate's stdio-only threat model) by
  the security review of this crate's own MA-6d PR, then mirrored back into
  `apl-repl` since both crates share the identical scanner design. An
  oversized line is now rejected cleanly (`Error: line exceeds the
  65536-byte limit; discarded`) with its remainder drained (one more
  bounded chunk) rather than left to be picked up mid-line by the next
  read.
- `read_bounded_line` reads raw bytes (`read_until(b'\n', ..)`) and only
  decodes UTF-8 after confirming the byte run ended at a genuine `\n`
  within the cap, rather than calling `BufRead::read_line` directly on the
  `Take`-wrapped reader — found by `/security-review` on this same change:
  `Take` truncates at an arbitrary byte offset with no notion of a
  character boundary, so a valid UTF-8 line only slightly over the cap
  could have a multi-byte character straddle that offset and make
  `read_line` report a spurious (and fatal — it aborted the whole session)
  `InvalidData` error instead of the intended clean "line too long"
  message. Ported from `apl-repl`'s identical fix.
- The "oversized?" check itself now requires **both** no trailing `\n`
  *and* the byte count actually reaching `MAX_LINE_LEN` — found by a
  second `/security-review` round on the fix above: `read_until` also
  stops with no trailing `\n` at genuine EOF (e.g. the very last line of
  input has no closing newline), and checking `\n`-absence alone
  misclassified that ordinary, short, valid line as "oversized" and
  silently discarded it. A real I/O error while draining an oversized
  line's remainder is now propagated rather than swallowed, for the same
  reason. Ported from `apl-repl`'s identical fix.
- The discard step now **loops** over as many further capped chunks as it
  takes to reach the oversized line's real end (a real `\n`) or true EOF,
  instead of draining exactly one extra chunk and stopping — found by a
  third `/security-review` round: a line whose true length spanned more
  than two cap-widths left its remaining tail (ending in a real `\n`)
  sitting in the reader, so the *next* read picked that fragment up and
  misinterpreted it as a fresh, independently-typed statement, defeating
  the "the whole oversized line is discarded" guarantee. The same round
  also confirmed a related boundary case is handled correctly: a final
  line whose length lands *exactly* on the cap, immediately followed by
  genuine EOF, is a maximal ordinary line, not an oversized one — resolved
  by checking for at least one more byte beyond the cap before reporting
  "oversized". Ported from `apl-repl`'s identical fix.

## [0.1.0] - 2026-07-13

### Added

- Initial release — the interactive REPL and the **`j` binary** (item
  MA-6d), a sibling of `apl-repl`/`matlab-repl`/`s-repl`/`r-repl` over the
  `j-runtime` interpreter.
- `JRepl` with a persistent workspace, `>> `/`... ` prompts, and `quit`/
  `exit`/EOF handling; `run()` drives a full stdio session.
- **Line continuation across an open `(`** — the only grouping construct in
  this language cut (no block keywords, no string type). Continuation lines
  are joined with a space rather than a real newline, since `j.tokens` does
  not drop newlines inside `(...)` in this first cut — mirrors `apl-repl`'s
  own identical scanner design.
- `NB.` line comments need zero REPL-level handling — they're stripped
  entirely at the lexer's skip-pattern level, exactly like APL's `⍝`.
- Errors are surfaced (`Error: …`) without ending the session.
- `MAX_CONTINUATION_BUFFER` (64 KiB) caps the pending-continuation buffer
  while a `(` is still unbalanced, discarding it with a clean error rather
  than growing without bound — ported forward from `apl-repl`'s own
  already-security-reviewed guard (same rationale: low severity under this
  crate's stdio-only threat model, kept anyway for defense in depth).
