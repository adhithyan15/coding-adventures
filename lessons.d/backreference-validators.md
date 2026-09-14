---
category: Cryptography & security review
---

# Backreference validators

: every LZ77/LZSS-style decoder needs `offset > 0` AND `offset <= output.length` checks before indexing into the decoded prefix. Throw `FormatException` on malformed/truncated streams.
