---
category: TypeScript / JavaScript
---

# Fence metadata must be sliced after the matched delimiter rather than rematched

The parser had already recognized an exact backtick or tilde opener, but a
second regex consumed both character kinds and used dot without dotAll. It
therefore removed opposite fence characters that belonged to metadata and
dropped info containing literal U+2028/U+2029. The earlier corpus lacked these
independent literals.

Pass the already-sliced metadata suffix to normalization. Avoid rediscovering
a delimiter with broader character classes or assuming regex line boundaries
match Markdown physical lines. Add opposite-prefix and Unicode separator
witnesses while retaining the complete literal CommonMark regression corpus.
