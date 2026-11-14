---
category: Security boundaries
---

# A one-line bug fixed in one language needs each port's encrypt audited too, not just the mirrored decrypt

SEC-010 was reported as "decrypt negates i32::MIN" in Rust. Porting that fix
to nine other languages showed the same bug is not always the same shape:

- Most ports (C#, F#, Java, Kotlin, Dart, Haskell, Lua) wrap on `-shift` and
  return wrong plaintext silently; their `encrypt` already reduced `% 26` first.
- Swift TRAPS on `-Int.min` (a crash, not a wrong answer).
- Go and Swift `encrypt` were ALSO unsafe: they added the raw shift to the
  letter offset before reducing, so `offset + shift` overflows near `MaxInt`.
  The Rust-shaped fix ("only fix decrypt") would have left that hole.

The durable rule: reduce an untrusted integer to its small residue FIRST, then
add, subtract or negate. When porting a fix, re-derive each port's overflow
surface from its own code (every arithmetic op touching the raw value), and pin
the by-hand expected value in the test (2^31 mod 26 = 24, 2^63 mod 26 = 8)
rather than computing it with the code under test.
