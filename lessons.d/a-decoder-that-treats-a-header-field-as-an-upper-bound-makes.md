---
category: Testing & coverage
---

# A decoder that treats a header field as an upper bound makes a corrupted header invisible to a round trip, so pin the encoder's exact output

**What went wrong.** The first `lzw.warrant` pinned `compress` only through `decompress(compress(d)) == Ok(d)` and a length check. The generated negative control `flip-result-bit` on `compress` was *accepted by every suite*: it flips a bit of the first byte of the result, which is the high byte of the 4-byte `original_length` header, and `decompress` treats that field as an upper bound (SEC-001 made it so on purpose: a stream that is shorter than it declares is not an error). The corrupted header declared 2 GiB, the decoder delivered the real data, and the round trip still held. The claim list looked complete (round trip, hostile inputs, bounds) and could not see a wrong header at all.

**Fix.** Claims that build the *entire* expected wire image from the specification and compare it with `compress`'s output byte for byte: every one-byte input (`CLEAR, x, STOP` at 9 bits, packed LSB first, header `00000001`) and every two-byte input (36 bits, 5 bytes, header `00000002`), 256 + 65,536 cases, plus known-answer examples from an independent implementation of the spec. The negative control is now caught by `compress_one_byte_matches_the_model`.

**Do differently.** A round trip proves the pair is consistent, not that either half is right, and a lenient decoder widens the gap (every field it ignores is a field a round trip cannot check). For each field the decoder tolerates, ask which claim fails when the encoder writes it wrongly; if the answer is "none", add a model-pinned claim on the encoder over a domain small enough to enumerate. Run the generated negative controls early: an uncaught control is a precise list of what the document does not say.
