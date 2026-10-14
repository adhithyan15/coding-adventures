---
category: Testing & coverage
---

# A checked geometry boundary can expose an older self loop normalization defect

A new checked Swimlane geometry boundary refused the existing ordering/reference
fixture because self-loop normalization already produced NaN endpoints. The old
control explicitly normalized matching NaNs before full equality, so ordinary
suite success had not meant finite geometry.

Keep the initial failed receipt and commit the finite-anchor contract first.
Handle coincident centers before division, retain all endpoint-bit and complete
reference assertions, and replace obsolete NaN masking with finite assertions.
Independent exact anchor controls prove the new branch instead of treating a
reference that calls the same renderer as independent correctness evidence.
