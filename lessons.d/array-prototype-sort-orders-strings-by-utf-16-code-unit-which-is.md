---
category: TypeScript / JavaScript
---

# Array.prototype.sort orders strings by UTF-16 code unit, which is not code point order for U+E000 to U+FFFF against astral characters

`Object.keys(o).sort()` compares UTF-16 code units, so U+1F600 (surrogates
`d83d de00`) sorts before U+FFFF, while Python's `sorted` and UTF-8 byte order put
U+FFFF first. The two orders agree below U+D800 and for BMP-only text, which is
why it survived review. Use a code point comparison (`Array.from(s, c =>
c.codePointAt(0))`) for any ordering that must match another language's. Cases
`json/key-order-bmp-vs-astral` and `json/key-order-private-use-vs-astral` pin it.
