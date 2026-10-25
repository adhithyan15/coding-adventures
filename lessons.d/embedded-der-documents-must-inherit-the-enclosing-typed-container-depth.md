---
category: Cryptography & security review
---

# Embedded DER documents must inherit the enclosing typed container depth

An extension-specific decoder initially opened `extnValue` with
`decode_exact`, which preserved the shared element counter but reset the
embedded document to depth zero. That lets nested input evade the caller's
composed depth budget even though every individual layer looks bounded.
Retain the validated OCTET STRING `Asn1Element` in the generic Extension value
and open its bytes with `decode_exact_embedded(container, bytes)`. Prove the
composition with one decoder: parse the outer Extension, then decode the
embedded structure under a depth limit that would pass if depth were reset but
must fail when the enclosing depth is carried forward.
