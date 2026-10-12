---
category: Rust
---

# Coincident curve controls need the first nonzero endpoint direction

Independent review withheld a Bump integration despite 754 passing diagram
tests. An authored LR bumpY route had controls equal to its endpoints; the
arrowhead normalization received a zero vector and emitted three identical
vertices. TB bumpX and aligned Step endpoint markers share the assumption.

Test the authored parser/layout/paint pipeline on every platform and assert
triangle vertices and area, alongside unchanged curve commands and metadata.
A cubic endpoint uses its first noncoincident control, then the next control,
then the opposite endpoint. Step routes use the first noncoincident waypoint.
Keep an entirely coincident route's existing behavior explicit. Marker presence
or a cubic command alone does not prove that the marker has visible geometry.

The first red runner selected an existing authored test as well as its new
controls: actual cargo reported 2PASS/3FAIL, while the wrapper expected 1PASS.
Keep that helper failure and assess the exact named failures from the saved log;
never replace actual execution counts with the intended filter's counts.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
