---
category: Cryptography & security review
---

# Recursive parsers (markdown, etc.) need depth + input-size caps at every recursive entry point

, not just the public API. Inline parsers that retry delimiter parsing char-by-char also need bounded unmatched-delimiter scans, or quadratic work on hostile input becomes a DoS.
