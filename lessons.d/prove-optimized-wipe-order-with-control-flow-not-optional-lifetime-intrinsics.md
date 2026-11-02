---
category: Cryptography & security review
---

# Prove optimized wipe order with control flow, not optional lifetime intrinsics

An optimized-IR regression gate required an exact `llvm.lifetime.end` spelling
after a cryptographic wipe. Hosted stable compilers may omit that optional
intrinsic or vary its pointer attributes even when the generated success path
is correct. Prove the security property through mandatory operations instead:
tie the exact tag payload to its full-length copy and require the exact wipe in
the same basic block. Treat lifetime intrinsics as debugging hints rather than
portable evidence: their presence, attributes, and printed block order are not
stable enough to gate the security property.
