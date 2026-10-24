---
category: Rust
---

# Packed-memory padding bits must be masked before narrow-register loads

The EDSAC store represents each long word as two host integers.  The even
cell contains a valid eighteenth "sandwich" bit, but a short operand contains
only the lower seventeen bits.  Loading the whole even cell into a short
multiplier operand let that packing-only bit become bit 15 of a 15-bit
register slice.  Execution succeeded, yet the resulting state could not pass
the simulator's own restore validation.

Mask a packed cell to the architectural width at the boundary where it is
loaded into a narrower register.  Tests for packed formats must deliberately
set every valid padding or separator bit and then verify both the computed
result and snapshot round-tripping.  A valid memory image must never produce
an internally invalid successful state.
