---
category: Testing & coverage
---

# .NET parallel test runs need isolated artifacts

: `dotnet test --artifacts-path .artifacts`. On Linux, ALSO set `HOME="$PWD/.dotnet"`, `DOTNET_CLI_HOME="$PWD/.dotnet"`, AND `TMPDIR="$PWD/.dotnet/tmp"` — the CLI's first-run `NuGet-Migrations` mutex uses `/tmp/.dotnet/shm` shared state that races otherwise.
