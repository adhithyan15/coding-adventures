# DB-E148 — Publish the Go NAND page-program cut model

Release the verified Go page-program transition and its shared
fixture in a standalone slice. The merged Go NAND simulator supplies
the media vocabulary. The next Go FTL release will depend on the
cut transition exposed by this package.
[Go page cuts](DB04b1-nand-page-program-cuts-go.md) · [Fixture scalar audit](DB-E128-go-nand-page-program-fixture-scalars.md)

Reconcile the release against the actual fetched main after the
current media release. Use the exact `go/nand-page-program-cuts`
package with the scalar fix from `9205450e89f`, its two specs and the
shared `nand-page-program-cuts.json` vector. Register the Go reader
against that exact file using the branch's existing repository
input registry. Generate the affected routing projections from that
base. Include only the package, specs, vector and required routing
changes in the PR.

Confirm that a fixture-only edit selects that reader and changes its
cache key. Compare every other graph row and emitted recipe with the
actual base. Run the emitted package recipe, the Go, C#, Swift and
Python input conformance suites, and fixture registry and corpus
checks. Verify that the controller or block-erase packages do not
enter this release as transitive requirements.

Use pre-push review against the exact publication head. After push,
verify the package ran on Linux, Windows and macOS and both required
gates passed. Confirm the merged tree. Then qualify the block-erase
cut package and the page-map primitives for their own releases.
