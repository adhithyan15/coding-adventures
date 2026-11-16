---
category: Testing & coverage
---

# A masked list is a claim that no input reaches an item, so it needs a large directed search before it exempts anything from a sensitivity gate

PHY00a 6.4 exempted 16 constants and 20 fused multiply-add sites from its
sensitivity gate on the grounds that "no sampled vector can see" them, backed by
reasoning about magnitudes and a modest sample. A 20-million-input search then found
inputs that DO see `C3` (about 1 in 15,000 inputs in [1/2, pi/4)), `S3`, `C4`, `S4`,
`C5` and the fused steps `sin.h2`, `sin.h3`, `cos.h2`, `cos.h3`; a follow-up of 1.2
billion inputs per item found `S5` (1 in 4e8, large arguments) and `cos.h4`. The
exemption was a false weakening: a port that mistyped `C3` would have passed.

The fix: pin directed vectors (found by a C re-implementation with a switchable
constant or fused site, `trig_reference_xcheck/search.c`) the way the reduction tie
cases already pin `red.t`; shrink the masked list to what a large search never hit;
and make the gate fail in BOTH directions, so a masked item that some vector does
detect is an error ("shrink the list") and an unmasked item must be caught at one of
its own witnesses, not by accident. (The accidental case was real: the `C5` witness
also detected `cos.h4`, and the gate said so.)

What to do differently: treat "cannot reach a result" as a hypothesis to falsify.
Argue magnitudes only to decide where to look, search the region where rounding
matters most (here the top of the reduced range, where `|r|` is largest), record the
search size and method next to the list, and let the generator keep the list honest.
