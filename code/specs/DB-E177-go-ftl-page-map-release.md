# DB-E177 — Publish the Go flash translation page map

Release the admitted logical page map, physical reservations
and reclaimed-page state. Carry the reviewed invalidation,
unowned physical program and settled snapshot operations in
their existing verification order.
[Page map](DB05a-ftl-page-map.md) · [Reclamation](DB05c2a-ftl-page-reclamation.md) · [Fixture routing](DB-E42-E45-ftl-page-map-fixture-routing.md) · [Scalar audit](DB-E177a-go-ftl-page-map-fixture-scalars.md)

Use the package revisions `5e203fc610d`, `67197ce5e40`,
`54db6a4c34c` and `d3ae98ed9e1`, and the exact scalar repair.
Carry the two canonical page-map and reclamation vectors.
Introduce each contract before its implementation. Regenerate
the routing projections from fetched main after the NAND erase
release.

Verify both vector paths independently, including individual
deletion, neighboring nonmatch, cache keys and Kiln inputs.
Compare the exact Go package, both inputs and recipe against
the existing graph and emitted steps. Run both build lists,
race, vet, tidy, guided inputs, clean controls and the four
build-tool fronts. Check native jobs, required gates and merged
bytes on fetched main.
