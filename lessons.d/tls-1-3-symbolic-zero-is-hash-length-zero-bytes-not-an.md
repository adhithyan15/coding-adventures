---
category: Cryptography & security review
---

# TLS 1.3 symbolic zero is Hash.length zero bytes, not an empty byte string

RFC 8446 key-schedule diagrams use `0` to mean a string of `Hash.length` zero
bytes. An `HKDF-Expand-Label` test initially translated the early-secret input
as an empty byte string, producing a different PRK and making the independent
RFC 8448 expected value disagree with otherwise-correct framing and expansion.
When transcribing TLS 1.3 key-schedule vectors, resolve every symbolic `0`
through the RFC definition before calling a byte-oriented primitive; do not
infer that it means an absent or zero-length input.
