---
category: Testing & coverage
---

# CollectCoverage needs a Coverlet MSBuild dependency

Passing `/p:CollectCoverage=true` to `dotnet test` is inert when the test
project does not reference `coverlet.msbuild`; the test run can succeed without
printing a coverage table or enforcing the requested threshold.  Include both
`coverlet.collector` and `coverlet.msbuild`, filter coverage to the assembly
under test, and verify that the build output contains Coverlet's coverage table.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
