---
category: Cryptography & security review
---

# Cryptographic fixture bytes must be verified against the signing trace before implementation tests

An OCSP signature fixture recorded the signed `ResponseData` digest correctly,
but its Rust hex constant accidentally duplicated one `0x22` issuer-key-hash
byte while wrapping the text. Every verifier test then failed in structural DER
decoding before reaching the intended cryptographic boundary.

For a signed fixture, compare the exact embedded payload bytes with the signing
trace before running verifier tests: assert the byte count and digest, then
decode the structure independently. A digest comment is useful evidence only
when the checked-in bytes are actually re-hashed and compared with it.
