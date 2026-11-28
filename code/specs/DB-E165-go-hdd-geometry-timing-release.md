# DB-E165 — Publish configurable Go HDD geometry, timing and cuts

Expose the pure placement, timing and sector-cut operations used by a
sparse disk on the Go block contract. Keep the existing 32-sector
simulator and crash-cut fixture available as reference cases.
[Geometry](DB06b2d1-go-hdd-geometry.md) · [Timing](DB06b2d2-go-hdd-seek-timing.md) · [Sector cuts](DB06b2d3a-go-hdd-sector-cut-description.md)

Publish the geometry package from `5febaad47dd`, the seek planner
from `cbba5e43dea`, and the cut description extension from the reviewed
`77fdbf77b59` change. Each specification precedes its implementation.
Reconcile the extension with the already published Go crash-cut package.
Preserve its exact fixture bytes, existing Go route and public behavior.

Use fresh exact release archives to test physical round trips, portable
ceilings, both geometry profiles, and each recognized sector cut. Run both
build lists, race, vet, tidy, guided inputs and clean named mutation
controls. Include the shared fixture at the path used by the native cut
suite. Recheck the resolved dependency and fixture graph and all metadata.
Verify both required gates and the emitted native build results before
conditional auto-merge. Compare the merged scope against the reviewed
head on fetched main.
