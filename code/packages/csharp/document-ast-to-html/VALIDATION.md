# C# HTML renderer validation

Local validation used .NET SDK 9.0.315 on Windows. The Unix recipe ran through
Git Bash; it is not evidence of execution on a Linux host. A short dedicated
`NUGET_PACKAGES` directory avoided missing restored assemblies in the initial
deep worktree cache. That initial bootstrap failure is not counted as a RED
regression result.

- Six public-API regressions failed with actual missing-exception assertions
  against the original renderer, then all six passed after the input-work fix.
  They cover aliased link/image destinations, discarded and blocked URLs,
  the inclusive limit, UTF-16 accounting, shared node-kind accounting and
  reset after successful or rejected calls.
- The complete renderer suite passed **33 tests, zero skips**, with **100%
  production line coverage and 98.82% branch coverage**. The Unix recipe and
  two consecutive executions of `BUILD_windows` reported these results. A
  successful Windows setup conditional alone is not accepted as execution.
- The existing document-AST dependency passed its four native tests. Its
  existing Windows front was not qualified for repeat execution, and no new
  dependency-coverage claim is made.
- `dotnet format --verify-no-changes` passed for the renderer and test projects.
  `dotnet pack` succeeded, and a separate consumer restored both locally built
  NuGet packages and checked literal HTML plus aggregate URL-budget rejection.
- The NuGet audit, including transitive test dependencies, reported no
  vulnerable packages from the configured sources.
- The unchanged neutral GFM corpus validated all 16 cases and its validator
  passed 11 tests. This is not native C# GFM or CommonMark qualification.
- The exact branch-tree parity report retained 15 established languages,
  1,536 identities and 4,819 slots, with no canonical collisions or unknown
  buckets. Directory presence does not establish behavioral parity.
- Repository lesson validation and `git diff --check` passed. Independent
  security review identified the URL-work issue in round one and passed the
  corrected production code in round two.

The renderer still borrows its input tree and returns no partial HTML after a
limit failure. These results do not close the separately owned 419 CommonMark
parser mismatches or qualify a future facade.
