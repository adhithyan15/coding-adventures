---
category: Testing & coverage
---

# A zero-exit dotnet command does not prove tests ran without executed results and host runtime evidence

An early `dotnet test --no-restore` check exited zero even though it was a build-only/no-op path
with no test assets and no executed test report. Treating that exit as a pass would have overstated
native projection coverage. A later real run produced 163 C# and 24 F# test results, but both
projects target .NET 9 and the local host was .NET 11 with `DOTNET_ROLL_FORWARD=Major`; the target
framework alone does not prove that .NET 9 executed.

For every test claim, preserve the command and exit code plus an execution artifact that reports
nonzero executed test counts (for example, TRX). When runtime identity matters, capture the actual
testhost runtime from diagnostics. Keep target framework, SDK, runtime, roll-forward settings, and
executed counts as separate facts. Do not count a successful build, an empty test run, or a target
framework declaration as native-runtime test evidence. Record local .NET 11 roll-forward runs as
such and leave native .NET 9 validation to a run that proves the .NET 9 host actually executed.
