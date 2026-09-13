---
category: Lua
---

# `^` returns float in Lua 5.4

— `2^24` is `16777216.0`, fails `math.type(x) == "integer"` checks. Use `1 << 24` (bitwise ops always return integers).
