---
category: Compiler / VM / language pipeline
---

# PC-relative addressing must name whether its base is the instruction address or the post-fetch PC

The first Data General Nova functional model used the simulator's PC after the
fetch increment as the base for PC-relative addressing. Its specification and
test repeated that assumption, so the green test fixed every relative target
one word too high. The primary programmer's reference instead defines the base
as the address of the instruction; an independent executable reference likewise
subtracts one from its already-incremented PC.

Carry the instruction address separately through decode whenever fetch advances
the stored PC before effective-address calculation. State the chosen base in the
specification and add direct and indirect regressions at address zero with a
negative displacement, where instruction-address and post-fetch bases produce
different 15-bit wraparound targets. A test whose expected address is copied
from the same lifecycle assumption is not an independent architecture witness.
