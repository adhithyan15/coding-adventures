# DB-E182 — Publish the Go interrupted-erase role qualifier

Release the admitted pure qualifier for page roles before an
interrupted erase. It matches the retained intent to the observed
block and erase generation. Its only shared dependency is the
Go FTL page-map role vocabulary.
[Role qualifier](DB05e3b3d-ftl-interrupted-erase-role-evidence.md) · [Geometry admission](DB05e3b3h2b1-ftl-erase-role-geometry-admission.md) · [Page-map release](DB-E177-go-ftl-page-map-release.md)

Carry `30448037587` and `3bcfab25039` in that order after the page-map
release. Introduce the qualifier and geometry contracts before their
changes. Preserve the module path, the exact `Free` and `Stale` role
values and the staged publication boundary. Verify that the sole
package dependency is the Go page map on each platform. The package
has no repository fixture input.

Repeat the independent geometry and retained-mask checks against the
exact release artifact. Exercise detached results and exact typed
refusals. Run both build lists, race, vet, tidy, Linux/386, the Go
fuzz target for 100,000 executions and the three existing mutation
controls. Compare the exact emitted recipe and the build graph
against the fetched base and check the native CI receipts and gates.
