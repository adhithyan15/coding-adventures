---
category: Cryptography & security review
---

# Embedded DER documents must inherit the enclosing typed container depth

An extension-specific decoder initially opened `extnValue` with
`decode_exact`, which preserved the shared element counter but reset the
embedded document to depth zero. That lets nested input evade the caller's
composed depth budget even though every individual layer looks bounded.
Retain opaque context derived from the validated OCTET STRING in the generic
Extension value and open its bytes with
`decode_exact_embedded_context(context, bytes)`. This carries only the depth
authority rather than duplicating full element framing in every bounded inline
extension slot. Prove the composition with one decoder: parse the outer
Extension, then decode the embedded structure under a depth limit that would
pass if depth were reset but must fail when the enclosing depth is carried
forward.
