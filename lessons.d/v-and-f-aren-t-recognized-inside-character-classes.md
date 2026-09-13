---
category: Lua
---

# `\v` and `\f` aren't recognized inside character classes

in Lua's regex engine — they're matched literally. Lua lexers loading `.tokens` grammars must replace them with actual control chars before parsing: `content:gsub("\\v", "\x0B"):gsub("\\f", "\x0C")`.
