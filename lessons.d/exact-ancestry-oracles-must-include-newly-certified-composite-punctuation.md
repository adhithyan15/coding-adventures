---
category: Rust
---

# Exact ancestry oracles must include newly certified composite punctuation

CLOC35's focused member tests passed, but the full CLI suite exposed an older
D2 string-length oracle that expected only the raw string. The newly certified
original member correctly adds the dot and property token through its own CV.

Declare the complete expected physical spans independently, retaining the raw
astral string's exact byte/scalar/UTF-16 coordinates. Require the unique member's
input, range and ordered parents plus the fold's exact immediate operand merge.
Keep whole-set equality excluding every unrelated token, trivia and EOF, and
leave the unary case unchanged. Do not relax exact ancestry to token presence.
