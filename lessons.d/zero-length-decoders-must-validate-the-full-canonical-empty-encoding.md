---
category: Cryptography & security review
---

# Zero-length decoders must validate the full canonical empty encoding

— don't early-return on declared length 0; check trailing bytes and end-of-block markers.
