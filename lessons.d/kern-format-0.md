---
category: QR / format-marker / file-format specifics
---

# `kern` Format 0

: subtable format is in the HIGH byte of `coverage` — `coverage >> 8 == 0`. `coverage & 0xFF == 0` checks flags, not format, and skips all valid Format 0 subtables (horizontal flag = bit 0 sets the low byte to 1).
