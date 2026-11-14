---
category: Security boundaries
---

# A port copies its original's missing bound, and may even document it as by design, so a fix to one port needs a sweep of every port

SEC-011: the Rust `irc-framing` `Framer::feed` appended every byte and only checked the 510-byte limit once a line ended in LF, so a peer that never sends LF grew the buffer without limit (CWE-400/770). Nine ports (c, cpp, csharp, fsharp, elixir, go, python, ruby, typescript) had copied the same shape. Two wrote the gap down as a feature: the c and cpp headers said the unbounded buffer was "by design" and the read layer must bound it, and the Elixir moduledoc claimed the 510-byte rule prevented memory exhaustion. The spec (`irc-framing.md`) had asked for a bound all along, but described it as "reset buffer, continue", which would re-frame the tail of a flooded line as a bogus short line.

What to do differently:

- When a security fix lands in one language, sweep every port of that package in the same change set and record a status per port, including the ones that turn out to be absent. A fix in the original does not reach the copies.
- Test the property, not the symptom: feed 16 MiB with no terminator, once in chunks and once in one call, and assert the buffer stays small. A per-line length check passes while that test fails.
- Read the comments before trusting them. A claim that a limit "prevents exhaustion" is a hypothesis to test, and "by design" next to an unbounded buffer is a bug report.
- Check the boundary on both sides: a pending line of 510 bytes plus CR must survive, 512 bytes must be dropped, and `reset()` must clear the skip-to-LF state or the next connection's first line vanishes.
