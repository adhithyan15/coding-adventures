---
category: Testing & coverage
---

# Portable tokenization must name the exact whitespace grammar

The portable GFM validator used Python `str.split()` to enforce a contract
defined in terms of ECMAScript whitespace. The sets differ: Python splits on
U+001C through U+001F and U+0085, while ECMAScript does not, and Python does not
split on U+FEFF, while ECMAScript does. ASCII-only fixtures hid the mismatch.

When a cross-language contract names a host language's lexical class, encode
that class explicitly instead of using a similarly named local helper. Test at
least one code point unique to each side of the difference, plus a Unicode
Space_Separator, so a future refactor cannot silently restore host semantics.
