---
category: QR / format-marker / file-format specifics
---

# Intel 4004 has no AND instruction

`AND_IMM vR, vR, 15` and `AND_IMM vR, vR, 255` are no-ops on a 4-bit machine; emit a comment, not an opcode. Other masks would need a RAM lookup table.
