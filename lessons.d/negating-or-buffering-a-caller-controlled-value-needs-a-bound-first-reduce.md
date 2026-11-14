---
category: Security boundaries
---

# Negating or buffering a caller-controlled value needs a bound first: reduce the shift before negating, cap the pending line before buffering

Two defects found by reading, both the same shape: an operation that is safe for
ordinary input was applied to a value an attacker or an extreme caller picks.

- SEC-010 (`caesar-cipher`): `decrypt` wrote `encrypt(text, -shift)`. `-i32::MIN`
  overflows: a panic in debug, a silent wrap in release. Fix: reduce first
  (`shift.rem_euclid(26)`), then negate; a reduced value cannot overflow.
- SEC-011 (`irc-framing`): `Framer::feed` kept every byte until a line feed
  showed up, and the 510-byte limit only applied to lines that had already
  ended. A peer that never sends LF grew the buffer without bound
  (CWE-400/770). Fix: cap the unterminated tail at 511 bytes and discard to the
  next LF.

What to do differently: when you write unary minus, `abs`, or a shift on a
fixed-width integer parameter, ask what `MIN` does. When a stream buffer waits
for a delimiter, ask what happens if the delimiter never comes, and put the
limit where bytes are stored, not where lines are parsed. The same two patterns
exist in the other language ports (see SEC-010 and SEC-011 in VER-BACKLOG), so
grep for the shape when you fix one. Found by reading the code; a test that
fails first (`decrypt(text, i32::MIN)`, 16 MiB with no LF) makes it real.
