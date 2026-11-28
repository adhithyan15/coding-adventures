# DB-E44 and DB-E52 — Route flash victim and evacuation fixtures

Add the two already verified Go planning packages after the page-map
fixture routes. Each package has its own fixture and exact consuming root.

| Rule | Exact input | Sole exact root |
|---|---|---|
| `repository-database-media-ftl-erase-victim-fixture-inputs` | `ftl-erase-victim.json` | `code/packages/go/ftl-erase-victim` |
| `repository-database-media-ftl-evacuation-plan-fixture-inputs` | `ftl-evacuation-plan.json` | `code/packages/go/ftl-evacuation-plan` |

Both vectors live under `code/specs/fixtures/database-media-v1/`.
Use the reviewed victim at `0872a35535ea` and planner at
`a9a6616bbb9f`. Pin package and fixture bytes from those revisions.
For each vector separately, verify direct selection with no language
filter and the Go filter, deletion with the exact path missing,
fallback hash invalidation and exact Kiln input. With both vectors
changed, select the two readers once each. Close paths, unrelated
database media fixtures and opposite-reader fixture edits preserve
each test hash.

The dependency graph resolves the actual module manifests. A page-map
source edit reaches both verified consumers. The two planner modules
have no dependency on each other, so changing one module reaches only
that planner at this stage. Preserve the page-map vector paths and
their existing hash behavior.

Synchronize the boundary registry and regenerate C# and Swift
projections. Re-run the full native Go, C# and Swift build-tool
checks, focused Python conformance, census synchronization and corpus.
Re-run the admitted page-map dependencies and both verified planners.
