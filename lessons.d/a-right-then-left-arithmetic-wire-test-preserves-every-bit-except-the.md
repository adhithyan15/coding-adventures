---
category: Testing & coverage
---

# A right-then-left arithmetic wire test preserves every bit except the global least-significant bit

An EDSAC gate-path test put a one in the least-significant bit of the most
significant 15-bit accumulator tank, shifted the complete 71-bit accumulator
right, shifted it left, and incorrectly expected that tank bit to disappear.
Only the complete register's least-significant wire is discarded by that
round-trip; the selected tank bit moves to the next wire and returns. Derive
shift expectations from the full concatenated register layout, not from the
local numbering of one host-side slice.
