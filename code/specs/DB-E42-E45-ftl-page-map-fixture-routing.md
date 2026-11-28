# DB-E42 and DB-E45 — Route both flash page-map vectors

The accepted Go `ftl-page-map` package at `a986cd06742d` reads both
the original reservation vector and the reclamation vector. Register
both fixture inputs in the same package delivery so either edit
selects the tested reader and invalidates its cached result.

| Rule | Exact input | Profile |
|---|---|---|
| `repository-database-media-ftl-page-map-fixture-inputs` | `ftl-page-map.json` | DB05a |
| `repository-database-media-ftl-page-reclamation-fixture-inputs` | `ftl-page-reclamation.json` | DB05c2a |

Both rules use `code/packages/go/ftl-page-map` as the sole exact root.
The vectors live under `code/specs/fixtures/database-media-v1/`.
Each vector change directly selects the page map. The package's
build input list and fallback hash include both paths. A nearby
path, ordinary NAND or HDD vector, and another FTL vector preserve
the page map's hash.

Check unfiltered and Go-only selection for each vector, deletion of
each exact path, and one change affecting both vectors at once.
Confirm the program and canonical registry agree on the combined
package input list. Preserve the admitted runtime and both vectors.
Later DB-E43, DB-E44 and DB-E52 exercise source propagation into
the actual dependent packages as those routes enter the stack.

Run boundary sync and check, native Go tests and vet, C# and Swift
conformance, focused Python tests and corpus validation. Re-run
the admitted page-map native tests with both exact inputs present.
