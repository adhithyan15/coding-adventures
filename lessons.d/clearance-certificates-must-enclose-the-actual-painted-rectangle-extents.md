---
category: Rust
---

# Clearance certificates must enclose the actual painted rectangle extents

Centered center+half and a painted rectangle's (center-half)+size can differ by
one ULP. The checked Ishikawa code validated both then discarded the latter,
so its centered4px certificate could overstate actual clearance. Enclose both
right/bottom operations and refuse candidates that cannot prove the actual gap;
keep font values and successful candidate arithmetic unchanged.

The first scalar witness also omitted max-depth-driven canvas height. Withdraw
that height360 claim; the corrected depth2 route has height400. Bind complete
public geometry and obtain actual Rust evidence before promoting a scalar model
to a runtime claim, and retain a same-font isolated-label positive control.
