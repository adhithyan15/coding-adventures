---
category: Rust
---

# A terminal conditional branch must distinguish taken control transfer from untaken fallthrough before exempting program-counter wrap

The EDSAC simulator initially exempted every conditional branch opcode from
the terminal sequence-control overflow check. That was too broad: a branch at
location 1023 can be untaken, in which case its ordinary fallthrough would
silently wrap the ten-bit program counter to zero.

Track whether the branch condition actually transferred control. Exempt only
a taken branch (and a halt), not the opcode family. Tests must cover both
conditions at the highest address, including a legitimate taken branch whose
target is zero; checking only the final program-counter value cannot
distinguish that transfer from an accidental wrap.
