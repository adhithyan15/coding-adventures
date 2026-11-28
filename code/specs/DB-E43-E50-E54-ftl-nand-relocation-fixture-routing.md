# DB-E43, DB-E50 and DB-E54 — Route all NAND relocation vectors

Admit the Go flash relocation package at its reviewed single-page copy
revision, `eac57c06fba0`, including the DB-E51 token checks in the
original write fixture reader and the DB-E57 block fuzz verdict repair at
`5381f690b40b`. The full tested package directly reads three vectors.

| Rule | Exact input | Profile |
|---|---|---|
| `repository-database-media-ftl-nand-relocation-fixture-inputs` | `ftl-nand-relocation.json` | DB05b |
| `repository-database-media-ftl-block-reclamation-fixture-inputs` | `ftl-block-reclamation.json` | DB05c2b |
| `repository-database-media-ftl-live-page-copy-fixture-inputs` | `ftl-live-page-copy.json` | DB05c3b1 |

Use `code/packages/go/ftl-nand-relocation` as the sole exact root for
each vector under `code/specs/fixtures/database-media-v1/`. Pin the
runtime and each fixture from their reviewed revision before starting.
The exact package input list contains all three, deterministically
ordered. A change or deletion of any vector selects Go relocation
directly and invalidates its fallback hash. A change to all three
selects one root and keeps one input entry per path. Include nearby
paths and other NAND and FTL fixture vectors as controls.

Exercise the actual package dependency graph. Verify propagation into
relocation from the Go map, NAND program and NAND erase packages, the
erase-victim module used by the relocation tests, and the evacuation
planner. Check that the three relocation vectors remain direct inputs
of the relocation package independently of those source edges.

Synchronize the boundary registry and regenerate C# and Swift
projections. Run the native and corpus build-tool checks and re-run
relocation against all three fixtures together with its dependencies.
