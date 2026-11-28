# DB-E61 — Route the copy-aware FTL victim fixture

Add the exact input
`code/specs/fixtures/database-media-v1/ftl-copy-aware-victim.json`
for the Go package at `code/packages/go/ftl-erase-victim`.
The package reads both its original stale-only fixture and this DB05c3c
fixture. Use the admitted selector and composition test at
`5161ef9b114` with the stale-only fixture reader correction at
`232a2ceee18`. Preserve both vectors. The original vector SHA-256 is
`1b8a3d941bcf8857f2e2d0167f4871e31d98fd3cc53d39d80f9986818b454f1d`.
The copy-aware vector SHA-256 is
`f24badf998303b5a92b6ac05ca8b389c0551942006a9b3022addb1218c0f1c25`.

Create a separate boundary rule
`repository-database-media-ftl-copy-aware-victim-fixture-inputs`
with that single package root, one input with role
`cross_package_exact`, and no descendant or excluded roots. Use the
existing ownership of the build-tool shared-input contract.

Check the original and new paths independently on both change and
deletion. Each selects the Go victim package once and belongs in its
direct hash and Kiln inputs. A concurrent change to both selects the
reader once and lists each exact input once. Verify that adjacent paths
and other FTL vectors preserve the package's direct hash. Verify that
a relevant page-map source change reaches the victim and that the
victim reaches relocation via its test dependency.

Regenerate the registry projections and pins. Keep both media vectors
and the admitted package unchanged. Re-run the Go, C#, Swift, Python,
corpus and exact selector checks.
