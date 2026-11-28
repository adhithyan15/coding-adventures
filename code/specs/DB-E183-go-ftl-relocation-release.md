# DB-E183 — Publish Go NAND-backed FTL relocation

Release the verified Go relocation device and four canonical fixture
readers. The device retains the current readable owner through
incomplete programs, publishes completed moves, fences active
operations, and uses retained role evidence when reopening an
interrupted erase.
[Relocation fixtures](DB-E43-E50-E54-ftl-nand-relocation-fixture-routing.md) · [Scalar repair](DB-E178-go-ftl-relocation-fixture-scalars.md) · [Planning release](DB-E181-go-ftl-planning-release.md) · [Erase role evidence release](DB-E182-go-ftl-erase-role-evidence-release.md)

Use the exact runtime at `0f15cc956e5` with the qualified test-only
scalar repair. Reconcile it on fetched main after the Go page map,
planners, NAND program and erase models, and erase-role qualifier.
Preserve the reviewed source and vector byte ordering when composing
the release and include the contracts for the exposed device operations.

The Go package directly reads `ftl-nand-relocation.json`,
`ftl-block-reclamation.json`, `ftl-live-page-copy.json` and
`ftl-evacuation.json`. Verify change and deletion of each exact path,
cache-key isolation, both module and fixture dependencies in the graph,
and the emitted native recipe. Keep prior registry rules exact and
regenerate every affected build-tool front.

Repeat the four canonical replays and the archived independent
operation histories. Run both lists, race, vet, tidy, 386, each
parser fuzz target and all current named controls. Confirm exact
selected CI package results and required gates before publishing.
