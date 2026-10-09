# G2D00-G2D03 neutral edge-case fixtures v1

This process-free corpus freezes point normalization, affine composition and
inversion, quadratic/cubic Bezier algebra, and SVG arc-degeneracy seams in
[G2D00](../../G2D00-point2d.md), [G2D01](../../G2D01-affine2d.md),
[G2D02](../../G2D02-bezier2d.md), and [G2D03](../../G2D03-arc2d.md). It is
not a claim that every existing lane already conforms. Native flattening repairs use the
separate `bezier2d-flattening-v1` adversarial corpus.

`point-normalize` takes a two-number point and expects a two-number point.
`svg-arc-degenerate` takes endpoint pairs, radii, and a sample parameter; its
center conversion must be absent, cubic list empty, evaluation equal to
`expected_point`, and ordered bounds equal to `[x, y, width, height]`.
The x-rotation is zero, large-arc false, and sweep true for these degenerate
cases because those flags cannot change a line fallback. All numbers must be
finite. The `comparison` field requires adapters to compare a
`point-normalize` expected origin exactly,
coordinate by coordinate: the sub-threshold input `5e-13` must not pass merely
because it is within the general `1e-12` output tolerance. Compare other
numeric outputs with the pinned absolute tolerance (no relative tolerance),
while checking presence, list length, and field shape exactly.

`affine-compose` takes two six-number matrices in `[a,b,c,d,e,f]` order and
a point; `first.multiply(second)` applies the second matrix first. Both
matrix and transformed-point outputs are pinned. `affine-invert` publishes a
six-number inverse or exact `null` under the strict `abs(det) < 1e-12`
singularity threshold. `affine-vector` excludes translation by construction.

`bezier-quadratic` and `bezier-cubic` take three or four control points and a
sample `t`. Each publishes evaluation, tangent derivative, the two complete
split control polygons, and tight `[x,y,width,height]` bounds from derivative
roots. One cubic case places its x maximum beyond both endpoints, ensuring a
conservative control-point box or endpoint-only box cannot pass. The validator
derives polynomial answers independently of native implementations and rejects
changed expectations, incomplete shapes, duplicate IDs/keys, and tolerance
drift. Existing native readers may filter by operation; adding a new family
does not imply that an older reader consumes it.

`center-arc-bounds` pins analytic `[x,y,width,height]` bounds for two
off-grid extrema across the $2\pi$ seam (positive and negative sweep), a
zero-sweep point rect, and rotated interior extrema. Adapters must account
for angle periodicity; sampling 100 points does not satisfy these cases.
`center-arc-cubics` pins one segment
for zero sweep, four for a full turn, and fail-stop `invalid-sweep` for a span
exceeding one turn. All center-form inputs must be finite, positive-radius,
and bounded before any angle arithmetic or allocation. JSON cannot represent
NaN or infinity, so native readers need separate non-finite rejection tests.
Existing native readers that filter by operation may continue to consume only
their earlier families until the separate lane-conformance owners land.

The independent validator recomputes every expected point and rectangle from
the spec's formulas without calling a package implementation:

```sh
python code/scripts/geometry2d_conformance.py
python -m unittest discover -s code/scripts/tests -p test_geometry2d_conformance.py -v
```

These two commands also run in the unconditional repo-wide metadata-contracts
CI step on every pull request. This checks the neutral corpus and oracle, not
the separate native reader conformance of each implementation lane.

SVG's negative-radius absolute-value rule and zero-radius line rule come from
the [W3C SVG 1.1 implementation notes](https://www.w3.org/TR/SVG11/implnote.html#ArcOutOfRangeParameters).
