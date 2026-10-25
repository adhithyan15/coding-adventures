---
category: Compiler / VM / language pipeline
---

# Read all architectural operands before writing an overlapping destination

System/360 `BXH` and `BXLE` add an increment to the first operand and compare
the result with a second register. The first implementation wrote the result
before reading that comparison register. Legal overlapping encodings therefore
compared against the new result rather than the original operand value.

Decode register numbers first, then snapshot every architectural source before
performing any destination write. Conformance tests must deliberately alias the
destination with each source role, including instruction-specific odd/even
register selection rules; disjoint-register tests cannot expose read-after-write
ordering bugs.
