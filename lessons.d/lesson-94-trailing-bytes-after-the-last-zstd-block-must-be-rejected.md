# Lesson 94 — Trailing bytes after the last ZStd block must be rejected, not silently ignored

**Date:** 2026-04-26

**What happened:** The Lua ZStd decoder iterated blocks in a `while true` loop and broke on `last_block == 1`. Any bytes remaining in the input after the last block were silently ignored. A fuzz input consisting of a valid 5-byte frame followed by 1 MB of garbage would be accepted without complaint, masking corruption and making the decoder lenient about malformed or concatenated frames.

**Rule:** After the block-decoding loop exits (when `last_block == 1`), assert that the read cursor equals `#data` (or `data.length`, or the frame boundary). If any bytes remain, raise an error:

```lua
if pos <= #data then
  error("unexpected trailing data after last block")
end
```

The same check belongs in every language port. A strict decoder is far safer — it surfaces truncation and concatenation bugs immediately rather than silently returning partial output or accepting garbage.

---
