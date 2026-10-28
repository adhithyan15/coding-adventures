# bezier2d (Go)

G2D02 quadratic and cubic Bezier evaluation, derivatives, splitting,
conservative polyline flattening, tight bounds, and quadratic degree elevation
over the shared `point2d` primitives.

`PolylineQuad` and `PolylineCubic` return `(points, error)`. Flattening accepts
only finite controls and a finite positive tolerance, measures every control
point against the finite endpoint segment, and preserves parameter order
without duplicate joins. It fails atomically if derived geometry becomes
non-finite or work would exceed depth 32 or 65,535 subdivisions.

Run the native quality gates with:

```sh
go test ./... -cover
go vet ./...
```
