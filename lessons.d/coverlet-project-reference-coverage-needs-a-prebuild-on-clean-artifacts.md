---
category: Testing & coverage
---

# Coverlet project-reference coverage needs a prebuild on clean artifacts

: A single `dotnet test` with MSBuild Coverlet can pass every test yet report
0% for a referenced production assembly on the first invocation against a
clean `--artifacts-path`; the incremental rerun then reports the real coverage.
Build the test project graph first, then run the same test project with
`--no-build`, `CollectCoverage=true`, an assembly-scoped `Include`, and the
threshold. Verify the build front once from absent artifacts, not only from a
warm output tree.
