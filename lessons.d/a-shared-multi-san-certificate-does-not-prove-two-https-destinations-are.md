---
category: Cryptography & security review
---

# A shared multi-SAN certificate does not prove two HTTPS destinations are the same

Two independently constructed destination proofs can retain the same exact
decoded certificate while selecting different DNS names that are both valid
subject alternative names. Certificate object identity alone therefore cannot
bind a prepared request destination to a reviewed server-path destination.

When composing the proofs, require the exact certificate object and compare
both retained SNI names and both retained Host authorities under their defined
DNS semantics, including the optional Host port. Keep the complete inputs in
the result so later code cannot detach those identity checks from either proof.
