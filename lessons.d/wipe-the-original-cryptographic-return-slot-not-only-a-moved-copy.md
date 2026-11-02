---
category: Cryptography & security review
---

# Wipe the original cryptographic return slot, not only a moved copy

Wrapping a returned authentication tag in a wipe-on-drop owner did not scrub
the ABI result slot that held the tag first: optimized code copied the bytes
into the wrapper, wiped that second buffer, and ended the original slot's
lifetime unchanged. Keep the successful tag borrowed in its original result
storage and protect that borrow with a drop guard. Pair the source-level design
with an optimized-IR gate that ties each result payload and previous-secret
owner to the repository zeroize primitive before accepting a wiping claim.
