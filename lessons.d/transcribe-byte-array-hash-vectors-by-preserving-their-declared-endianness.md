---
category: Testing & coverage
---

# Transcribe byte-array hash vectors by preserving their declared endianness

The Swift SipHash test initially copied a familiar 64-bit hexadecimal vector
from memory and changed one byte (`0x63` became `0x76`). The implementation
passed the first 27 upstream rows before the incorrect expectation failed.

When an upstream oracle publishes each digest as a byte array, copy those exact
bytes first and derive the display integer using the algorithm's declared byte
order. Compare any hand-grouped hexadecimal literal back to the source bytes;
do not treat recognition of the surrounding reference-vector sequence as
evidence that an individual literal was transcribed correctly.
