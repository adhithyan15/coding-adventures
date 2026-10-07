# G2D00/G2D03 neutral edge-case fixtures v1

This process-free corpus freezes the point-normalization and SVG arc-degeneracy
seams in [G2D00](../../G2D00-point2d.md) and
[G2D03](../../G2D03-arc2d.md). It is not a claim that every existing lane
already conforms, nor that rotated non-degenerate bounds are reconciled.

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

The independent validator recomputes every expected point and rectangle from
the spec's formulas without calling a package implementation:

```sh
python code/scripts/geometry2d_conformance.py
python -m unittest discover -s code/scripts/tests -p test_geometry2d_conformance.py -v
```

SVG's negative-radius absolute-value rule and zero-radius line rule come from
the [W3C SVG 1.1 implementation notes](https://www.w3.org/TR/SVG11/implnote.html#ArcOutOfRangeParameters).
