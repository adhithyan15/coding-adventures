---
category: Cryptography & security review
---

# Variable-length integer encoding with a format-marker byte: marker MUST come first

on the wire, regardless of host endianness. Zstd seq_count: `(count >> 8) | 0x80` BEFORE `count & 0xFF`. Round-trip tests on a self-consistent broken codec are blind to byte-order bugs — always include integration tests with values in each form whose low byte is < 128.
