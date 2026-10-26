---
category: Cryptography & security review
---

# Verify the exact hash algorithm and message bytes before embedding a familiar known-answer digest

A familiar-looking 384-bit digest was initially attached to the standard
112-byte SHA-2 message without independently checking that exact algorithm and
byte string. The implementation produced a different result even though its
empty and `abc` vectors passed. An independent platform SHA-384 implementation
confirmed the code's result and exposed the expected value as the mistaken
fixture.

Before embedding a cryptographic known-answer value, hash the exact fixture
bytes with an independent implementation and record both the byte count and
algorithm. Do not infer that a remembered digest belongs to the right SHA-2 or
SHA-3 variant merely because its width and prefix look plausible.
