---
category: TypeScript / JavaScript
---

# JS bitwise ops are signed 32-bit

`1 << 32 === 1` (shifts are mod 32) — guard `bitWidth >= 32` separately. `0xFFFFFFFF & 0xFFFFFFFF === -1` — use `>>> 0` to convert to unsigned: `(value & mask) >>> 0`. Critical for register files / ALU / addressing.
